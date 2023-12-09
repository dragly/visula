use std::time::Instant;

use crate::camera::Camera;
use crate::custom_event::CustomEvent;
use crate::rendering_descriptor::RenderingDescriptor;
use crate::{camera::controller::CameraController, simulation::RenderData};

use egui::{Context, FontDefinitions};
use wgpu::{Color, CommandEncoder, InstanceDescriptor, SurfaceTexture, TextureView};
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    window::Window,
};

use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

use crate::{CameraControllerResponse, Simulation};

pub struct Application {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface,
    pub window: Window,
    pub camera_controller: CameraController,
    pub depth_texture: wgpu::TextureView,
    pub multisampled_framebuffer: wgpu::TextureView,
    pub platform: Platform,
    pub egui_rpass: RenderPass,
    pub camera: Camera,
    pub start_time: Instant,
}

impl Application {
    pub async fn new(window: Window) -> Application {
        let size = window.inner_size();

        // TODO remove this when https://github.com/gfx-rs/wgpu/issues/1492 is resolved
        let backends = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();
        let gles_minor_version = wgpu::util::gles_minor_version_from_env().unwrap_or_default();

        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends,
            dx12_shader_compiler,
            flags: wgpu::InstanceFlags::from_build_config().with_env(),
            gles_minor_version,
        });
        let surface = unsafe { instance.create_surface(&window).unwrap() };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    #[cfg(target_arch = "wasm32")]
                    limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    #[cfg(not(target_arch = "wasm32"))]
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let mut config = surface
            .get_default_config(&adapter, size.width, size.height)
            .expect("Surface isn't supported by the adapter.");
        let surface_view_format = config.format.add_srgb_suffix();
        config.view_formats.push(surface_view_format);
        surface.configure(&device, &config);

        let camera_controller = CameraController::new(&window);

        let depth_texture_in = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        });
        let depth_texture = depth_texture_in.create_view(&wgpu::TextureViewDescriptor::default());

        let multisampled_framebuffer = Self::create_multisampled_framebuffer(&device, &config, 4);

        let mut platform = Platform::new(PlatformDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });
        let start_time = Instant::now();
        platform.update_time(start_time.elapsed().as_secs_f64());

        let egui_rpass = RenderPass::new(&device, config.format, 1);

        let camera = Camera::new(&device);

        Application {
            device,
            queue,
            config,
            camera_controller,
            surface,
            window,
            depth_texture,
            multisampled_framebuffer,
            camera,
            platform,
            egui_rpass,
            start_time,
        }
    }

    fn create_multisampled_framebuffer(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        sample_count: u32,
    ) -> wgpu::TextureView {
        let multisampled_texture_extent = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: config.view_formats[0],
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn handle_event(
        &mut self,
        event: &Event<CustomEvent>,
        control_flow: &mut ControlFlow,
    ) -> bool {
        self.platform
            .update_time(self.start_time.elapsed().as_secs_f64());

        if let Event::WindowEvent {
            window_id,
            event: _event,
        } = event
        {
            if *window_id != self.window.id() {
                return false;
            }
        }
        self.platform.handle_event(event);
        if self.platform.captures_event(event) {
            return true;
        }
        let CameraControllerResponse {
            needs_redraw,
            captured_event,
        } = self.camera_controller.handle_event(event);
        if needs_redraw {
            self.window.request_redraw();
        }
        if captured_event {
            return true;
        }
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
                println!("{}", crude_profiler::report());
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    size: wgpu::Extent3d {
                        width: size.width,
                        height: size.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 4,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    label: None,
                    view_formats: &[],
                });
                self.depth_texture =
                    depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
                self.config.width = size.width;
                self.config.height = size.height;
                self.multisampled_framebuffer =
                    Self::create_multisampled_framebuffer(&self.device, &self.config, 4);
                self.surface.configure(&self.device, &self.config);
            }
            _ => {}
        }
        false
    }

    pub fn update(&mut self) {
        self.camera_controller.update();
        let camera_uniforms = self
            .camera_controller
            .uniforms(self.config.width as f32 / self.config.height as f32);
        self.camera.update(&camera_uniforms, &self.queue);
    }

    pub fn next_frame(&self) -> SurfaceTexture {
        match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .expect("Failed to acquire next surface texture!")
            }
        }
    }

    pub fn encoder(&self) -> CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
    }

    pub fn begin_render_pass(
        &self,
        frame: &SurfaceTexture,
        encoder: &mut CommandEncoder,
        clear_color: Color,
    ) -> TextureView {
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.multisampled_framebuffer,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }
        view
    }

    pub fn begin_egui_frame(&mut self) -> Context {
        self.platform.begin_frame();
        self.platform.context()
    }

    pub fn end_egui_frame(&mut self, frame: &SurfaceTexture, encoder: &mut CommandEncoder) {
        let output_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let full_output = self.platform.end_frame(Some(&self.window));
        let paint_jobs = self.platform.context().tessellate(full_output.shapes);

        let screen_descriptor = ScreenDescriptor {
            physical_width: self.config.width,
            physical_height: self.config.height,
            scale_factor: self.window.scale_factor() as f32,
        };
        self.egui_rpass
            .add_textures(&self.device, &self.queue, &full_output.textures_delta)
            .unwrap();
        self.egui_rpass
            .update_buffers(&self.device, &self.queue, &paint_jobs, &screen_descriptor);

        self.egui_rpass
            .execute(encoder, &output_view, &paint_jobs, &screen_descriptor, None)
            .unwrap();
    }

    pub fn render(&mut self, simulation: &mut impl Simulation) {
        let frame = self.next_frame();
        let mut encoder = self.encoder();

        {
            let view = self.begin_render_pass(&frame, &mut encoder, simulation.clear_color());
            simulation.render(&mut RenderData {
                view: &view,
                multisampled_framebuffer: &self.multisampled_framebuffer,
                depth_texture: &self.depth_texture,
                encoder: &mut encoder,
                camera: &self.camera,
            });
        }

        {
            let egui_ctx = self.begin_egui_frame();
            simulation.gui(&egui_ctx);
            self.end_egui_frame(&frame, &mut encoder);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    pub fn rendering_descriptor(&self) -> RenderingDescriptor {
        RenderingDescriptor {
            device: &self.device,
            format: &self.config.format,
            camera: &self.camera,
        }
    }
}
