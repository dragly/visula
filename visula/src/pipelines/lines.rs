use crate::rendering_descriptor::RenderingDescriptor;
use crate::{DefaultRenderPassDescriptor, RenderData};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use naga::back::wgsl::WriterFlags;
use naga::valid::ValidationFlags;
use std::cell::Ref;
use std::mem::size_of;
use visula_core::{BindingBuilder, BufferBinding, Expression};
use visula_derive::Delegate;
use wgpu::BufferUsages;
use wgpu::{util::DeviceExt, BindGroupLayout};

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    length_weight: f32,
    width_weight: f32,
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

fn vertex(length_weight: f32, width_weight: f32) -> Vertex {
    Vertex {
        length_weight,
        width_weight,
    }
}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        vertex(0.0, 0.0),
        vertex(0.0, 1.0),
        vertex(1.0, 1.0),
        vertex(1.0, 0.0),
    ];

    let index_data: &[u16] = &[0, 1, 2, 2, 3, 0];

    (vertex_data.to_vec(), index_data.to_vec())
}

pub struct Lines {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: usize,
    binding_builder: BindingBuilder,
}

#[derive(Delegate)]
pub struct LineDelegate {
    pub start: Expression,
    pub end: Expression,
    pub width: Expression,
    pub alpha: Expression,
}

impl Default for LineDelegate {
    fn default() -> Self {
        LineDelegate {
            start: Vec3::new(0.0, 0.0, 0.0).into(),
            end: Vec3::new(1.0, 0.0, 0.0).into(),
            width: 1.0.into(),
            alpha: 1.0.into(),
        }
    }
}

impl Lines {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        delegate: &LineDelegate,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let &RenderingDescriptor {
            device,
            format,
            camera,
        } = rendering_descriptor;
        let mut module =
            naga::front::wgsl::parse_str(include_str!("../shaders/line.wgsl")).unwrap();
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 1);

        log::info!("Injecting line shader delegate");
        delegate.inject("line_input", &mut module, &mut binding_builder);

        let vertex_size = size_of::<Vertex>();
        let (vertex_data, index_data) = create_vertices();
        let index_count = index_data.len();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX | BufferUsages::COPY_DST,
        });

        log::debug!("Validating line shader\n{:#?}", module);
        let info =
            naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
                .validate(&module)
                .unwrap();
        let output_str =
            naga::back::wgsl::write_string(&module, &info, WriterFlags::all()).unwrap();
        log::debug!("Resulting lines shader code:\n{}", output_str);

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&output_str)),
        });

        let bind_group_layouts: Vec<&BindGroupLayout> = binding_builder
            .uniforms
            .values()
            .map(|binding| binding.bind_group_layout.as_ref())
            .collect();

        let uniforms = {
            let mut uniforms = vec![&camera.bind_group_layout];
            for layout in &bind_group_layouts {
                uniforms.push(layout);
            }
            uniforms
        };
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lines pipeline layout"),
            bind_group_layouts: &uniforms,
            push_constant_ranges: &[],
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 4,
                    shader_location: 1,
                },
            ],
        };
        let mut layouts = binding_builder
            .bindings
            .values()
            .map(|binding| binding.layout.build())
            .collect();
        let buffers = {
            let mut buffers = vec![vertex_buffer_layout];
            buffers.append(&mut layouts);
            buffers
        };
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lines render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: *format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
            multiview: None,
        });
        Ok(Lines {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
            binding_builder,
        })
    }

    pub fn render(
        &self,
        RenderData {
            encoder,
            view,
            depth_texture,
            camera,
            ..
        }: &mut RenderData,
    ) {
        log::trace!("Rendering lines");
        let mut count = None;
        for binding in self.binding_builder.bindings.values() {
            let other = binding.inner.borrow().count;
            if other == 0 {
                count = None;
                break;
            }
            count = match count {
                None => Some(other),
                Some(old) => {
                    if other != old {
                        None
                    } else {
                        Some(old)
                    }
                }
            }
        }
        log::trace!("Line count {count:#?}");
        if count.is_none() {
            log::debug!("Empty line buffer detected. Aborting render of lines.");
            return;
        }

        let bindings: Vec<(&BufferBinding, Ref<wgpu::Buffer>)> = self
            .binding_builder
            .bindings
            .values()
            .map(|v| {
                (
                    v,
                    Ref::map(v.inner.borrow(), |v| {
                        v.buffer.as_ref().expect("Buffer not set!")
                    }),
                )
            })
            .collect();
        let uniforms: Vec<Ref<wgpu::BindGroup>> = self
            .binding_builder
            .uniforms
            .values()
            .map(|v| Ref::map(v.inner.borrow(), |m| &m.bind_group))
            .collect();
        {
            let default_render_pass =
                DefaultRenderPassDescriptor::new("lines", view, depth_texture);
            let mut render_pass = encoder.begin_render_pass(&default_render_pass.build());
            render_pass.set_bind_group(0, &camera.bind_group, &[]);

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            let mut instance_count = 0;
            for (binding, buffer) in bindings.iter() {
                let slot = binding.slot;
                log::trace!("Setting vertex buffer {}", slot);
                render_pass.set_vertex_buffer(slot, buffer.slice(..));
                instance_count = instance_count.max(binding.inner.borrow().count);
            }
            for bind_group in uniforms.iter() {
                log::trace!("Setting bind group {}", 1);
                render_pass.set_bind_group(1, bind_group, &[]);
            }
            render_pass.draw_indexed(0..self.index_count as u32, 0, 0..instance_count as u32);
        }
    }
}
