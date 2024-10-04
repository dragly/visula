use crate::camera::Camera;
use crate::custom_event::CustomEvent;
use crate::rendering_descriptor::RenderingDescriptor;
use crate::{camera::controller::CameraController, simulation::RenderData};
use crate::{CameraControllerResponse, Simulation};
use chrono::{DateTime, Utc};
use egui::{Context, ViewportId};
use egui_wgpu::Renderer;
use egui_wgpu::ScreenDescriptor;
use egui_winit::State;

use std::sync::Arc;
use wgpu::{
    Color, CommandEncoder, Device, InstanceDescriptor, SurfaceTexture, TextureFormat, TextureView,
    TextureViewDescriptor,
};
use winit::{
    event::{Event, WindowEvent},
    window::Window,
};

pub struct Application {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub window: Arc<Window>,
    pub camera_controller: CameraController,
    pub depth_texture: wgpu::TextureView,
    pub multisampled_framebuffer: wgpu::TextureView,
    pub egui_renderer: EguiRenderer,
    pub egui_ctx: egui::Context,
    pub camera: Camera,
    pub start_time: DateTime<Utc>,
}

fn create_egui_context() -> egui::Context {
    pub const IS_DESKTOP: bool = cfg!(any(
        target_os = "freebsd",
        target_os = "linux",
        target_os = "macos",
        target_os = "openbsd",
        target_os = "windows",
    ));

    let egui_ctx = egui::Context::default();
    egui_ctx.set_embed_viewports(!IS_DESKTOP);
    egui_ctx
}

pub struct EguiRenderer {
    pub state: State,
    pub renderer: Renderer,
}

impl EguiRenderer {
    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> EguiRenderer {
        let egui_context = Context::default();
        let egui_state = egui_winit::State::new(
            egui_context,
            ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
        );
        //egui_state.set_pixels_per_point(window.scale_factor() as f32);
        let egui_renderer = egui_wgpu::Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
        );

        EguiRenderer {
            state: egui_state,
            renderer: egui_renderer,
        }
    }
}

impl Application {
    pub async fn new(window: Arc<Window>) -> Application {
        let size = window.inner_size();

        #[cfg(target_arch = "wasm32")]
        let backends = wgpu::Backends::GL;
        #[cfg(not(target_arch = "wasm32"))]
        let backends = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);

        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();
        let gles_minor_version = wgpu::util::gles_minor_version_from_env().unwrap_or_default();

        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends,
            dx12_shader_compiler,
            flags: wgpu::InstanceFlags::from_build_config().with_env(),
            gles_minor_version,
        });
        let surface = instance.create_surface(window.clone()).unwrap();
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
                    required_features: wgpu::Features::empty(),
                    #[cfg(target_arch = "wasm32")]
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    #[cfg(not(target_arch = "wasm32"))]
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let mut config = surface
            .get_default_config(&adapter, size.width.max(640), size.height.max(480))
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

        let start_time = Utc::now();

        let camera = Camera::new(&device);

        let egui_ctx = create_egui_context();
        let egui_renderer = EguiRenderer::new(&device, surface_view_format, None, 1, &window);

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
            egui_renderer,
            egui_ctx,
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
            view_formats: &config.view_formats,
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn handle_event(&mut self, event: &Event<CustomEvent>) -> bool {
        if let Event::WindowEvent {
            window_id,
            event: window_event,
        } = event
        {
            if *window_id != self.window.id() {
                return false;
            }
            let response = self
                .egui_renderer
                .state
                .on_window_event(&self.window, window_event);
            if response.consumed {
                return true;
            }
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

    pub fn clear(&self, view: &TextureView, encoder: &mut CommandEncoder, clear_color: Color) {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.multisampled_framebuffer,
                resolve_target: Some(view),
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

    pub fn render(&mut self, simulation: &mut impl Simulation) {
        let frame = self.next_frame();
        let mut encoder = self.encoder();

        let view = frame.texture.create_view(&TextureViewDescriptor {
            format: Some(self.config.view_formats[0]),
            ..wgpu::TextureViewDescriptor::default()
        });

        {
            self.clear(&view, &mut encoder, simulation.clear_color());
            simulation.render(&mut RenderData {
                view: &view,
                multisampled_framebuffer: &self.multisampled_framebuffer,
                depth_texture: &self.depth_texture,
                encoder: &mut encoder,
                camera: &self.camera,
            });
            let raw_input = self.egui_renderer.state.take_egui_input(&self.window);
            let full_output = self.egui_renderer.state.egui_ctx().run(raw_input, |ui| {
                simulation.gui(self, ui);
            });

            self.egui_renderer
                .state
                .handle_platform_output(&self.window, full_output.platform_output);

            let tris = self
                .egui_renderer
                .state
                .egui_ctx()
                .tessellate(full_output.shapes, full_output.pixels_per_point);
            for (id, image_delta) in &full_output.textures_delta.set {
                self.egui_renderer.renderer.update_texture(
                    &self.device,
                    &self.queue,
                    *id,
                    image_delta,
                );
            }
            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: full_output.pixels_per_point,
            };
            self.egui_renderer.renderer.update_buffers(
                &self.device,
                &self.queue,
                &mut encoder,
                &tris,
                &screen_descriptor,
            );
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.egui_renderer
                .renderer
                .render(&mut render_pass, &tris, &screen_descriptor);
            drop(render_pass);
            for x in &full_output.textures_delta.free {
                self.egui_renderer.renderer.free_texture(x)
            }
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
