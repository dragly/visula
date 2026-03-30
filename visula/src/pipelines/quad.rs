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
    pub fragment_shader_variable_name: Option<&'a str>,
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
    vertex_binding_builder: BindingBuilder,
    fragment_binding_builder: Option<BindingBuilder>,
}

impl QuadPipeline {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        descriptor: &QuadPipelineDescriptor,
        vertex_delegate: &dyn Delegate,
        fragment_delegate: Option<&dyn Delegate>,
    ) -> Result<Self, visula_core::ShaderError> {
        let &RenderingDescriptor {
            device,
            camera,
            format,
            ..
        } = rendering_descriptor;

        let shader_with_lighting = format!(
            "{}\n{}",
            visula_core::LIGHTING_WGSL,
            descriptor.shader_source,
        );
        let mut module = naga::front::wgsl::parse_str(&shader_with_lighting)?;
        let mut vertex_binding_builder = BindingBuilder::new(&module, "vs_main", 1)?;

        vertex_delegate.inject(
            descriptor.shader_variable_name,
            &mut module,
            &mut vertex_binding_builder,
        )?;

        let fragment_binding_builder = match (
            &fragment_delegate,
            &descriptor.fragment_shader_variable_name,
        ) {
            (Some(delegate), Some(variable_name)) => {
                let mut builder = BindingBuilder::new(&module, "fs_main", 0)?;
                delegate.inject_before_return(variable_name, &mut module, &mut builder)?;
                Some(builder)
            }
            _ => None,
        };

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
                .map_err(Box::new)?;
        let output_str = naga::back::wgsl::write_string(&module, &info, WriterFlags::all())?;
        log::debug!("Resulting {} shader code:\n{output_str}", descriptor.label);

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&output_str)),
        });

        let vertex_uniform_layouts: Vec<&BindGroupLayout> = vertex_binding_builder
            .uniforms
            .values()
            .map(|binding| binding.bind_group_layout.as_ref())
            .collect();

        let fragment_uniform_layouts: Vec<&BindGroupLayout> = fragment_binding_builder
            .as_ref()
            .map(|b| {
                b.uniforms
                    .values()
                    .map(|binding| binding.bind_group_layout.as_ref())
                    .collect()
            })
            .unwrap_or_default();

        let fragment_texture_layouts: Vec<BindGroupLayout> = fragment_binding_builder
            .as_ref()
            .map(|b| {
                b.textures
                    .values()
                    .map(|binding| binding.inner.borrow().bind_group_layout.clone())
                    .collect()
            })
            .unwrap_or_default();

        let bind_group_layouts = {
            let mut layouts = vec![&camera.bind_group_layout];
            for layout in &vertex_uniform_layouts {
                layouts.push(layout);
            }
            for layout in &fragment_texture_layouts {
                layouts.push(layout);
            }
            for layout in &fragment_uniform_layouts {
                layouts.push(layout);
            }
            layouts
        };

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{} pipeline layout", descriptor.label)),
            bind_group_layouts: &bind_group_layouts,
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
        let sorted_bindings = vertex_binding_builder.sorted_bindings();
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
            vertex_binding_builder,
            fragment_binding_builder,
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
        for binding in self.vertex_binding_builder.instances.values() {
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
        if count.is_none() && !self.vertex_binding_builder.instances.is_empty() {
            log::debug!("Empty {} buffer detected. Aborting render.", self.label);
            return;
        }

        let bindings: Vec<(&InstanceBinding, Ref<wgpu::Buffer>)> = self
            .vertex_binding_builder
            .instances
            .values()
            .map(|v| (v, Ref::map(v.inner.borrow(), |v| &v.buffer)))
            .collect();
        let vertex_uniforms: Vec<wgpu::BindGroup> = self
            .vertex_binding_builder
            .uniforms
            .values()
            .map(|v| v.inner.borrow().bind_group.clone())
            .collect();
        let fragment_textures: Vec<wgpu::BindGroup> = self
            .fragment_binding_builder
            .as_ref()
            .map(|b| {
                b.textures
                    .values()
                    .map(|v| v.inner.borrow().bind_group.clone())
                    .collect()
            })
            .unwrap_or_default();
        let fragment_uniforms: Vec<wgpu::BindGroup> = self
            .fragment_binding_builder
            .as_ref()
            .map(|b| {
                b.uniforms
                    .values()
                    .map(|v| v.inner.borrow().bind_group.clone())
                    .collect()
            })
            .unwrap_or_default();
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
            let mut instance_count = if self.vertex_binding_builder.instances.is_empty() {
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
            let mut current_slot = 1u32;
            for bind_group in vertex_uniforms.iter() {
                log::trace!("Setting vertex uniform bind group {current_slot}");
                render_pass.set_bind_group(current_slot, bind_group, &[]);
                current_slot += 1;
            }
            for bind_group in fragment_textures.iter() {
                log::trace!("Setting fragment texture bind group {current_slot}");
                render_pass.set_bind_group(current_slot, bind_group, &[]);
                current_slot += 1;
            }
            for bind_group in fragment_uniforms.iter() {
                log::trace!("Setting fragment uniform bind group {current_slot}");
                render_pass.set_bind_group(current_slot, bind_group, &[]);
                current_slot += 1;
            }
            render_pass.draw_indexed(0..self.index_count as u32, 0, 0..instance_count as u32);
        }
    }
}
