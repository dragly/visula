use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::pipelines::instanced::InstancedPipeline;
use crate::pipelines::Pipeline;
use crate::primitives::sphere::Sphere;
use crate::vertex_attr::VertexAttr;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    _pos: [f32; 3],
    _tex_coord: [f32; 2],
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
    Vertex {
        _pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32],
        _tex_coord: [tc[0] as f32, tc[1] as f32],
    }
}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        vertex([-1, -1, -1], [0, 0]),
        vertex([1, -1, -1], [1, 0]),
        vertex([1, 1, -1], [1, 1]),
        vertex([-1, 1, -1], [0, 1]),
    ];

    let index_data: &[u16] = &[0, 1, 2, 2, 3, 0];

    (vertex_data.to_vec(), index_data.to_vec())
}

pub struct Spheres {
    instanced_pipeline: InstancedPipeline,
}

impl Spheres {
    pub fn new(application: &crate::Application) -> Result<Self, Box<dyn std::error::Error>> {
        let crate::Application {
            device,
            camera_bind_group_layout,
            ..
        } = application;
        let vertex_size = size_of::<Vertex>();
        let (vertex_data, index_data) = create_vertices();
        let index_count = index_data.len();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_data = vec![Sphere {
            position: [0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0],
            radius: 1.0,
        }];

        let instance_count = instance_data.len();

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });

        let shader_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "../shader.wgsl"
            ))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0,
                shader_location: 0,
            }],
        };
        let instance_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<Sphere>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Sphere::attributes(1),
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_layout, instance_buffer_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[application.config.format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
        });

        Ok(Spheres {
            instanced_pipeline: InstancedPipeline {
                render_pipeline,
                vertex_buffer,
                index_buffer,
                index_count,
                instance_buffer,
                instance_count,
            },
        })
    }

    pub fn update<I>(&mut self, application: &crate::Application, sphere_iterator: I)
    where
        I: IntoIterator<Item = Sphere>,
    {
        let spheres: Vec<Sphere> = sphere_iterator.into_iter().collect();
        if spheres.len() == self.instanced_pipeline.instance_count {
            application.queue.write_buffer(
                &self.instanced_pipeline.instance_buffer,
                0,
                bytemuck::cast_slice(&spheres),
            );
        } else {
            self.instanced_pipeline.instance_buffer =
                application
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Instance buffer"),
                        contents: bytemuck::cast_slice(&spheres),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });
            self.instanced_pipeline.instance_count = spheres.len();
        }
        application.window.request_redraw();
    }

    pub fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.instanced_pipeline.render(render_pass);
    }
}
