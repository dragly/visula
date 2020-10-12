use crate::camera_controller::CameraController;
use crate::camera_uniforms::CameraUniforms;
use crate::custom_event::CustomEvent;
use crate::drop_event::DropEvent;
use crate::pipeline::Pipeline;
use crate::sphere::Sphere;
use crate::vec_to_buffer::vec_to_buffer;
use crate::Point3;
use cgmath::EuclideanSpace;

use std::mem::size_of;
use std::path::PathBuf;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    window::Window,
};

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
}

impl Application {
    pub fn handle_zdf(&mut self, path: &PathBuf) {
        let name: &str = path.to_str().unwrap().as_ref();
        let file = netcdf::open(name).unwrap();
        let group = &file.group("data").unwrap().unwrap();
        let pointcloud = &group
            .variable("pointcloud")
            .expect("Could not find pointcloud");

        let points = pointcloud.values::<f32>(None, None).unwrap();

        let mut mean_position = Point3::new(0.0, 0.0, 0.0);
        assert!(points.shape()[2] == 3);
        let shape = (points.shape()[0] * points.shape()[1], points.shape()[2]);
        let points_flat = points.into_shape(shape).unwrap();
        let instance_data: Vec<Sphere> = (&points_flat)
            .outer_iter()
            .filter_map(|point| {
                let x = point[0];
                let y = point[1];
                let z = point[2];
                if x.is_nan() || y.is_nan() || z.is_nan() {
                    return None;
                }
                let position = Point3::new(x, y, z);
                let (color, radius) = (Point3::new(1.0, 0.0, 0.0), 1.0);

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
        println!("{:?}", self.camera_controller);

        let instance_buffer = self.device.create_buffer_with_data(
            bytemuck::cast_slice(&instance_data),
            wgpu::BufferUsage::VERTEX,
        );

        // TODO there is a way to update an existing buffer instead of creating a new one
        self.points.instance_buffer = instance_buffer;
        self.points.instance_count = instance_count;
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

        let instance_buffer = self.device.create_buffer_with_data(
            bytemuck::cast_slice(&instance_data),
            wgpu::BufferUsage::VERTEX,
        );

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
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    label: None,
                });
                self.depth_texture = depth_texture.create_default_view();
                self.sc_desc.width = size.width;
                self.sc_desc.height = size.height;
            }
            Event::RedrawRequested(_) => {
                let frame = match self.swap_chain.get_next_frame() {
                    Ok(frame) => frame,
                    Err(_) => {
                        self.swap_chain =
                            self.device.create_swap_chain(&self.surface, &self.sc_desc);
                        self.swap_chain
                            .get_next_frame()
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

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.output.view,
                            resolve_target: None,
                            load_op: wgpu::LoadOp::Clear,
                            store_op: wgpu::StoreOp::Store,
                            clear_color: wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            },
                        }],
                        depth_stencil_attachment: Some(
                            wgpu::RenderPassDepthStencilAttachmentDescriptor {
                                attachment: &self.depth_texture,
                                depth_load_op: wgpu::LoadOp::Clear,
                                depth_store_op: wgpu::StoreOp::Store,
                                stencil_load_op: wgpu::LoadOp::Clear,
                                stencil_store_op: wgpu::StoreOp::Store,
                                clear_depth: 1.0,
                                clear_stencil: 0,
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
                            "zdf" => self.handle_zdf(&path),
                            _ => {
                                log::warn!("Unsupported format {}", extension);
                            }
                        }
                    }
                }
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
