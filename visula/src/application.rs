use crate::camera::controller::CameraControllerResponse;
use crate::camera::Camera;
use crate::custom_event::CustomEvent;
use crate::rendering_descriptor::RenderingDescriptor;
use crate::{camera::controller::CameraController, simulation::RenderData};

use egui::FontDefinitions;
use wgpu::InstanceDescriptor;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    window::Window,
};

use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

use crate::Simulation;

pub struct Application {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface,
    pub window: Window,
    pub camera_controller: CameraController,
    pub depth_texture: wgpu::TextureView,
    pub platform: Platform,
    pub egui_rpass: RenderPass,
    pub camera: Camera,
}

impl Application {
    pub async fn new(window: Window) -> Application {
        let size = window.inner_size();

        // TODO remove this when https://github.com/gfx-rs/wgpu/issues/1492 is resolved
        let backends = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends,
            dx12_shader_compiler,
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
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        });
        let depth_texture = depth_texture_in.create_view(&wgpu::TextureViewDescriptor::default());

        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

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
            camera,
            platform,
            egui_rpass,
        }
    }

    pub fn handle_event(
        &mut self,
        event: &Event<CustomEvent>,
        control_flow: &mut ControlFlow,
    ) -> bool {
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
                    sample_count: 1,
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
                self.surface.configure(&self.device, &self.config);
            }
            _ => {}
        }
        false
    }

    pub fn update(&mut self) {
        self.camera_controller.update();
    }

    pub fn render(&mut self, simulation: &mut impl Simulation) {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => {
                // This error occurs when the app is minimized on Windows.
                // Silently return here to prevent spamming the console with:
                // "The underlying surface has changed, and therefore the swap chain must be updated"
                return;
            }
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture!")
            }
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let camera_uniforms = self
                .camera_controller
                .uniforms(self.config.width as f32 / self.config.height as f32);
            self.camera.update(&camera_uniforms, &self.queue);
        }

        {
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            {
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("clear"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(simulation.clear_color()),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_texture,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                });
            }

            simulation.render(&mut RenderData {
                view: &view,
                depth_texture: &self.depth_texture,
                encoder: &mut encoder,
                camera: &self.camera,
            });
        }

        {
            let output_view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            self.platform.begin_frame();

            let egui_ctx = self.platform.context();
            simulation.gui(&egui_ctx);

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
            self.egui_rpass.update_buffers(
                &self.device,
                &self.queue,
                &paint_jobs,
                &screen_descriptor,
            );

            self.egui_rpass
                .execute(
                    &mut encoder,
                    &output_view,
                    &paint_jobs,
                    &screen_descriptor,
                    None,
                )
                .unwrap();
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
