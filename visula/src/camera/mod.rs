use std::mem::size_of;

use crate::vec_to_buffer::vec_to_buffer;

use self::uniforms::CameraUniforms;

pub mod controller;
pub mod uniforms;

pub struct Camera {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
}

impl Camera {
    pub fn new(device: &wgpu::Device) -> Camera {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        size_of::<uniforms::CameraUniforms>() as u64
                    ),
                },
                count: None,
            }],
        });

        // TODO get the definition of the size of the camera uniforms into one place somehow
        let model_view_projection_matrix =
            [0.0; size_of::<uniforms::CameraUniforms>() / size_of::<f32>()];

        let uniform_buffer = vec_to_buffer(
            device,
            model_view_projection_matrix.as_ref(),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Camera {
            bind_group_layout,
            bind_group,
            uniform_buffer,
        }
    }

    pub fn update(&mut self, uniforms: &CameraUniforms, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[*uniforms]),
        );
    }
}
