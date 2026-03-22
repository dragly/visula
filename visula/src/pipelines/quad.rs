use crate::rendering_descriptor::RenderingDescriptor;
use crate::simulation::RenderData;
use crate::{DefaultRenderPassDescriptor, Renderable};
use itertools::Itertools;
use naga::{back::wgsl::WriterFlags, valid::ValidationFlags};
use std::cell::Ref;
use visula_core::{BindingBuilder, Delegate, InstanceBinding};
use wgpu::util::DeviceExt;
use wgpu::{BindGroupLayout, BufferUsages, PipelineCompilationOptions};

pub struct QuadPipelineDescriptor<'a> {
    pub label: &'a str,
    pub shader_source: &'a str,
    pub shader_variable_name: &'a str,
    pub vertex_data: &'a [u8],
    pub vertex_stride: usize,
    pub vertex_format: wgpu::VertexFormat,
    pub index_data: &'a [u8],
    pub index_format: wgpu::IndexFormat,
}

pub struct QuadPipeline {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: usize,
    index_format: wgpu::IndexFormat,
    label: String,
    binding_builder: BindingBuilder,
}

impl QuadPipeline {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        descriptor: &QuadPipelineDescriptor,
        delegate: &dyn Delegate,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let &RenderingDescriptor {
            device,
            camera,
            format,
            ..
        } = rendering_descriptor;

        let mut module = naga::front::wgsl::parse_str(descriptor.shader_source).unwrap();
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 1);

        delegate.inject(
            descriptor.shader_variable_name,
            &mut module,
            &mut binding_builder,
        );

        let index_count = match descriptor.index_format {
            wgpu::IndexFormat::Uint16 => descriptor.index_data.len() / 2,
            wgpu::IndexFormat::Uint32 => descriptor.index_data.len() / 4,
        };

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} vertex buffer", descriptor.label)),
            contents: descriptor.vertex_data,
            usage: wgpu::BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} index buffer", descriptor.label)),
            contents: descriptor.index_data,
            usage: wgpu::BufferUsages::INDEX | BufferUsages::COPY_DST,
        });

        log::debug!("Validating {} shader", descriptor.label);
        let info =
            naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
                .validate(&module)
                .unwrap();
        let output_str =
            naga::back::wgsl::write_string(&module, &info, WriterFlags::all()).unwrap();
        log::debug!("Resulting {} shader code:\n{output_str}", descriptor.label);

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
            label: Some(&format!("{} pipeline layout", descriptor.label)),
            bind_group_layouts: &uniforms,
            push_constant_ranges: &[],
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: descriptor.vertex_stride as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: descriptor.vertex_format,
                offset: 0,
                shader_location: 0,
            }],
        };
        let sorted_bindings = binding_builder.sorted_bindings();
        let mut layouts = sorted_bindings
            .iter()
            .map(|binding| binding.layout.build())
            .collect_vec();

        let buffers = {
            let mut buffers = vec![vertex_buffer_layout];
            buffers.append(&mut layouts);
            buffers
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{} render pipeline", descriptor.label)),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &buffers,
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: *format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
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
            multisample: wgpu::MultisampleState {
                count: rendering_descriptor.sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Ok(QuadPipeline {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
            index_format: descriptor.index_format,
            label: descriptor.label.to_string(),
            binding_builder,
        })
    }
}

impl Renderable for QuadPipeline {
    fn render(
        &self,
        RenderData {
            encoder,
            view,
            multisampled_framebuffer,
            depth_texture,
            camera,
            ..
        }: &mut RenderData,
    ) {
        log::trace!("Rendering {}", self.label);
        if self.index_count == 0 {
            return;
        }
        let mut count = None;
        for binding in self.binding_builder.instances.values() {
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
        log::trace!("{} count {count:#?}", self.label);
        if count.is_none() && !self.binding_builder.instances.is_empty() {
            log::debug!("Empty {} buffer detected. Aborting render.", self.label);
            return;
        }

        let bindings: Vec<(&InstanceBinding, Ref<wgpu::Buffer>)> = self
            .binding_builder
            .instances
            .values()
            .map(|v| (v, Ref::map(v.inner.borrow(), |v| &v.buffer)))
            .collect();
        let uniforms: Vec<wgpu::BindGroup> = self
            .binding_builder
            .uniforms
            .values()
            .map(|v| v.inner.borrow().bind_group.clone())
            .collect();
        {
            let default_render_pass = DefaultRenderPassDescriptor::new(
                &self.label,
                view,
                multisampled_framebuffer,
                depth_texture,
            );
            let mut render_pass = encoder.begin_render_pass(&default_render_pass.build());
            render_pass.set_bind_group(0, &camera.bind_group, &[]);

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_index_buffer(self.index_buffer.slice(..), self.index_format);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            let mut instance_count = if self.binding_builder.instances.is_empty() {
                1
            } else {
                0
            };
            for (binding, buffer) in bindings.iter() {
                let slot = binding.slot;
                log::trace!("Setting vertex buffer {slot}");
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
