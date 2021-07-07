use crate::camera::controller::CameraController;
use crate::camera::uniforms::CameraUniforms;
use crate::custom_event::CustomEvent;
use crate::drop_event::DropEvent;
use crate::pipeline::Pipeline;
use crate::vec_to_buffer::vec_to_buffer;

use std::mem::size_of;
use std::path::Path;

use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    window::Window,
};

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
    pub swap_chain: wgpu::SwapChain,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub surface: wgpu::Surface,
    pub window: Window,
    pub camera_controller: CameraController,
    pub points: Pipeline,
    pub depth_texture: wgpu::TextureView,
    pub draw_mode: DrawMode,
}

impl Application {
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::op_ref)]
    pub fn handle_zdf(&mut self, path: &Path) {
        let crate::io::zdf::ZdfFile {
            camera_center,
            instance_buffer,
            instance_count,
            mesh_vertex_buf,
            mesh_vertex_count,
        } = crate::io::zdf::read_zdf(path, &mut self.device);

        self.camera_controller.center = camera_center;
        self.points.instance_buffer = instance_buffer;
        self.points.instance_count = instance_count;
        self.points.mesh_vertex_buf = mesh_vertex_buf;
        self.points.mesh_vertex_count = mesh_vertex_count;
    }

    pub fn handle_xyz(&mut self, DropEvent { text, .. }: &DropEvent) {
        let crate::io::xyz::XyzFile {
            instance_buffer,
            instance_count,
        } = crate::io::xyz::read_xyz(text, &mut self.device);

        self.points.instance_buffer = instance_buffer;
        self.points.instance_count = instance_count;
    }

    pub fn handle_event(&mut self, event: &Event<CustomEvent>, control_flow: &mut ControlFlow) {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(CustomEvent::DropEvent(drop_event)) => {
                self.handle_xyz(drop_event);
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
                    usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                    label: None,
                });
                self.depth_texture =
                    depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
                self.sc_desc.width = size.width;
                self.sc_desc.height = size.height;
                self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
            }
            Event::RedrawRequested(_) => {
                let frame = match self.swap_chain.get_current_frame() {
                    Ok(frame) => frame,
                    Err(_) => {
                        self.swap_chain =
                            self.device.create_swap_chain(&self.surface, &self.sc_desc);
                        self.swap_chain
                            .get_current_frame()
                            .expect("Failed to acquire next swap chain texture!")
                    }
                };

                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                {
                    let model_view_projection_matrix =
                        self.camera_controller.model_view_projection_matrix(
                            self.sc_desc.width as f32 / self.sc_desc.height as f32,
                        );

                    let model_view_projection_matrix_ref = model_view_projection_matrix.as_ref();

                    let temp_buf = vec_to_buffer(
                        &self.device,
                        &model_view_projection_matrix_ref.to_vec(),
                        wgpu::BufferUsage::COPY_SRC,
                    );
                    encoder.copy_buffer_to_buffer(
                        &temp_buf,
                        0,
                        &self.points.uniform_buffer,
                        0,
                        size_of::<CameraUniforms>() as u64,
                    );
                }

                if self.draw_mode == DrawMode::Points {
                    // Draw points
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("points render pass"),
                        color_attachments: &[wgpu::RenderPassColorAttachment {
                            view: &frame.output.view,
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
                    rpass.set_pipeline(&self.points.render_pipeline);
                    rpass.set_bind_group(0, &self.points.bind_group, &[]);
                    rpass.set_vertex_buffer(0, self.points.vertex_buffer.slice(..));
                    rpass.set_index_buffer(
                        self.points.index_buffer.slice(..),
                        wgpu::IndexFormat::Uint16,
                    );
                    rpass.set_vertex_buffer(1, self.points.instance_buffer.slice(..));
                    rpass.draw_indexed(
                        0..self.points.index_count as u32,
                        0,
                        0..self.points.instance_count as u32,
                    );
                } else {
                    // Draw meshes
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("mesh render pass"),
                        color_attachments: &[wgpu::RenderPassColorAttachment {
                            view: &frame.output.view,
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
                    rpass.set_pipeline(&self.points.mesh_render_pipeline);
                    rpass.set_bind_group(0, &self.points.mesh_bind_group, &[]);
                    rpass.set_vertex_buffer(0, self.points.mesh_vertex_buf.slice(..));
                    rpass.draw(0..self.points.mesh_vertex_count as u32, 0..1);
                }

                self.queue.submit(Some(encoder.finish()));
            }
            Event::WindowEvent {
                event: WindowEvent::DroppedFile(path),
                ..
            } => {
                log::info!("Dropped file {:?}", path);
                let bytes = std::fs::read(path).unwrap();
                let drop_event = DropEvent {
                    name: path.to_str().unwrap().to_string(),
                    text: bytes,
                };
                if let Some(extension) = path.extension() {
                    if let Some(extension) = extension.to_str() {
                        match extension {
                            "xyz" => self.handle_xyz(&drop_event),
                            #[cfg(not(target_arch = "wasm32"))]
                            "zdf" => self.handle_zdf(path),
                            _ => {
                                log::warn!("Unsupported format {}", extension);
                            }
                        }
                    }
                }
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
}
