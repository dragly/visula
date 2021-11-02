use crate::camera::controller::CameraController;
use crate::custom_event::CustomEvent;

use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    window::Window,
};

use crate::Simulation;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DrawMode {
    Points,
    Mesh,
}

impl Default for DrawMode {
    fn default() -> Self {
        DrawMode::Points
    }
}

pub struct Application {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface,
    pub window: Window,
    pub camera_controller: CameraController,
    pub camera_uniform_buffer: wgpu::Buffer,
    pub depth_texture: wgpu::TextureView,
    pub draw_mode: DrawMode,
    pub camera_bind_group: wgpu::BindGroup,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    // TODO make private
    pub next_buffer_handle: u64,
}

impl Application {
    pub fn handle_event(&mut self, event: &Event<CustomEvent>, control_flow: &mut ControlFlow) {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
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
                });
                self.depth_texture =
                    depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
                self.config.width = size.width;
                self.config.height = size.height;
                self.surface.configure(&self.device, &self.config);
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            winit::event::KeyboardInput {
                                virtual_keycode: Some(winit::event::VirtualKeyCode::M),
                                state: winit::event::ElementState::Pressed,
                                ..
                            },
                        ..
                    },
                ..
            } => {
                self.draw_mode = match self.draw_mode {
                    DrawMode::Mesh => DrawMode::Points,
                    DrawMode::Points => DrawMode::Mesh,
                };
                self.window.request_redraw();
            }
            Event::WindowEvent {
                event: window_event,
                ..
            } => {
                if self.camera_controller.handle_event(window_event) {
                    self.window.request_redraw();
                }
            }
            Event::MainEventsCleared => {
                // handle logic updates, such as physics
            }
            _ => {}
        }
    }

    pub fn render<S>(&mut self, simulation: &mut S)
    where
        S: Simulation,
    {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
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
            let model_view_projection_matrix = self
                .camera_controller
                .model_view_projection_matrix(self.config.width as f32 / self.config.height as f32);
            self.queue.write_buffer(
                &self.camera_uniform_buffer,
                0,
                bytemuck::cast_slice(&[model_view_projection_matrix]),
            );
        }

        {
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            simulation.render(&mut render_pass);

        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    pub fn create_buffer_handle(&mut self) -> u64 {
        let handle = self.next_buffer_handle;
        self.next_buffer_handle += 1;
        handle
    }
}
