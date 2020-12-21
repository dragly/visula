use crate::camera_controller::CameraController;
use crate::camera_uniforms::CameraUniforms;
use crate::custom_event::CustomEvent;
use crate::drop_event::DropEvent;
use crate::mesh::MeshVertexAttributes;
use crate::pipeline::Pipeline;
use crate::sphere::Sphere;
use crate::vec_to_buffer::vec_to_buffer;
use crate::Point3;
use cgmath::EuclideanSpace;

use wgpu::util::DeviceExt;

use ndarray::s;
use std::path::PathBuf;
use std::{iter::Iterator, mem::size_of};
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
    pub fn handle_zdf(&mut self, path: &PathBuf) {
        let name: &str = path.to_str().unwrap().as_ref();
        let file = netcdf::open(name).unwrap();
        let group = &file.group("data").unwrap().unwrap();
        let pointcloud = &group
            .variable("pointcloud")
            .expect("Could not find pointcloud");
        let rgba_image = &group
            .variable("rgba_image")
            .expect("Could not find pointcloud");

        let mut vertices = vec![];
        let points = pointcloud.values::<f32>(None, None).unwrap();
        let colors = rgba_image.values::<f32>(None, None).unwrap();
        for col in 0..(points.shape()[0] - 1) {
            for row in 0..(points.shape()[1] - 1) {
                let col_m = (col as i64 - 1).max(0) as usize;
                let row_m = (row as i64 - 1).max(0) as usize;
                let col_p = (col as i64 + 1).min(points.shape()[0] as i64 - 1) as usize;
                let row_p = (row as i64 + 1).min(points.shape()[1] as i64 - 1) as usize;

                let color = colors.slice(s![col, row, ..]);
                let color = [color[0] as u8, color[1] as u8, color[2] as u8, 255];
                let point_c = points.slice(s![col, row, ..]);
                let point_l = points.slice(s![col_m, row, ..]);
                let point_r = points.slice(s![col_p, row, ..]);
                let point_t = points.slice(s![col, row_m, ..]);
                let point_b = points.slice(s![col, row_p, ..]);
                let point_tl = points.slice(s![col_m, row_m, ..]);
                let point_bl = points.slice(s![col_m, row_p, ..]);
                let point_tr = points.slice(s![col_p, row_m, ..]);
                let point_br = points.slice(s![col_p, row_p, ..]);

                let corner_tr = (&point_r + &point_tr + &point_c + &point_t) / 4.0;
                let corner_tl = (&point_l + &point_tl + &point_c + &point_t) / 4.0;
                let corner_br = (&point_r + &point_br + &point_c + &point_b) / 4.0;
                let corner_bl = (&point_l + &point_bl + &point_c + &point_b) / 4.0;
                if !corner_tr[0].is_nan()
                    && !corner_tl[0].is_nan()
                    && !corner_br[0].is_nan()
                    && !corner_bl[0].is_nan()
                {
                    vertices.push(MeshVertexAttributes {
                        position: [corner_tr[0], corner_tr[1], corner_tr[2]],
                        color,
                        normal: [1.0, 0.0, 0.0],
                    });
                    vertices.push(MeshVertexAttributes {
                        position: [corner_tl[0], corner_tl[1], corner_tl[2]],
                        color,
                        normal: [1.0, 0.0, 0.0],
                    });
                    vertices.push(MeshVertexAttributes {
                        position: [corner_br[0], corner_br[1], corner_br[2]],
                        color,
                        normal: [1.0, 0.0, 0.0],
                    });
                    vertices.push(MeshVertexAttributes {
                        position: [corner_tl[0], corner_tl[1], corner_tl[2]],
                        color,
                        normal: [1.0, 0.0, 0.0],
                    });
                    vertices.push(MeshVertexAttributes {
                        position: [corner_bl[0], corner_bl[1], corner_bl[2]],
                        color,
                        normal: [1.0, 0.0, 0.0],
                    });
                    vertices.push(MeshVertexAttributes {
                        position: [corner_br[0], corner_br[1], corner_br[2]],
                        color,
                        normal: [1.0, 0.0, 0.0],
                    });
                }
            }
        }

        let mut mean_position = Point3::new(0.0, 0.0, 0.0);
        assert!(points.shape()[2] == 3);
        let points_shape = (points.shape()[0] * points.shape()[1], points.shape()[2]);
        let colors_shape = (colors.shape()[0] * colors.shape()[1], colors.shape()[2]);
        let points_flat = points.into_shape(points_shape).unwrap();
        let colors_flat = colors.into_shape(colors_shape).unwrap();
        let instance_data: Vec<Sphere> = points_flat
            .outer_iter()
            .zip(colors_flat.outer_iter())
            .filter_map(|(point, color)| {
                let x = point[0];
                let y = point[1];
                let z = point[2];
                if x.is_nan() || y.is_nan() || z.is_nan() {
                    return None;
                }
                let position = Point3::new(x, y, z);
                let color = Point3::new(color[0] / 255.0, color[1] / 255.0, color[2] / 255.0);
                let radius = 1.0;

                mean_position += position.to_vec();

                Some(Sphere {
                    position,
                    radius,
                    color,
                })
            })
            .collect();
        let instance_count = instance_data.len();

        self.camera_controller.center = (mean_position / instance_count as f32).to_vec();

        let instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsage::VERTEX,
            });

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsage::VERTEX,
            });

        println!("{:?}", self.camera_controller);

        // TODO there is a way to update an existing buffer instead of creating a new one
        self.points.instance_buffer = instance_buffer;
        self.points.instance_count = instance_count;
        self.points.mesh_vertex_buf = vertex_buffer;
        self.points.mesh_vertex_count = vertices.len();
    }
    pub fn handle_xyz(&mut self, DropEvent { name, text, .. }: &DropEvent) {
        log::info!("Got a drop {}", name);
        let text_bytes = text;
        let inner = std::io::Cursor::new(&text_bytes[..]);
        let mut trajectory = trajan::xyz::XYZReader::<f32, std::io::Cursor<&[u8]>>::new(
            trajan::coordinate::CoordKind::Position,
            inner,
        );
        let snapshot = trajectory.read_snapshot().unwrap();

        let instance_data: Vec<Sphere> = snapshot
            .particles
            .iter()
            .filter_map(|particle| {
                let position = match particle.xyz {
                    trajan::coordinate::Coordinate::Position { x, y, z } => {
                        Some(Point3::new(x, y, z))
                    }
                    _ => None,
                };
                println!("Name {}", particle.name);
                let (color, radius) = match particle.name.as_ref() {
                    "1" => (Point3::new(1.0, 0.0, 0.0), 1.0),
                    "2" => (Point3::new(0.0, 1.0, 0.0), 0.4),
                    _ => (Point3::new(1.0, 1.0, 1.0), 0.6),
                };
                match position {
                    Some(p) => Some(Sphere {
                        position: p,
                        radius,
                        color,
                    }),
                    _ => None,
                }
            })
            .collect();

        let instance_count = instance_data.len();

        let instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsage::VERTEX,
            });

        // TODO there is a way to update an existing buffer instead of creating a new one
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
                        depth: 1,
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
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.output.view,
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
                        depth_stencil_attachment: Some(
                            wgpu::RenderPassDepthStencilAttachmentDescriptor {
                                attachment: &self.depth_texture,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(1.0),
                                    store: true,
                                }),
                                stencil_ops: None,
                            },
                        ),
                    });
                    rpass.set_pipeline(&self.points.render_pipeline);
                    rpass.set_bind_group(0, &self.points.bind_group, &[]);
                    rpass.set_index_buffer(self.points.index_buffer.slice(..));
                    rpass.set_vertex_buffer(0, self.points.vertex_buffer.slice(..));
                    rpass.set_vertex_buffer(1, self.points.instance_buffer.slice(..));
                    rpass.draw_indexed(
                        0..self.points.index_count as u32,
                        0,
                        0..self.points.instance_count as u32,
                    );
                } else {
                    // Draw meshes
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.output.view,
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
                        depth_stencil_attachment: Some(
                            wgpu::RenderPassDepthStencilAttachmentDescriptor {
                                attachment: &self.depth_texture,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(1.0),
                                    store: true,
                                }),
                                stencil_ops: None,
                            },
                        ),
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
                            "zdf" => self.handle_zdf(&path),
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
                if self.camera_controller.handle_event(&window_event) {
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
