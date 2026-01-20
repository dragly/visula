use std::cell::Ref;
use std::mem::size_of;

use naga::back::wgsl::WriterFlags;
use naga::valid::ValidationFlags;
use wgpu::util::DeviceExt;
use wgpu::{BindGroupLayout, PipelineCompilationOptions};

use crate::primitives::mesh_primitive::MeshVertexAttributes;
use crate::{DefaultRenderPassDescriptor, RenderData, RenderingDescriptor};
use visula_core::{BindingBuilder, Expression, InstanceBinding};
use visula_derive::Delegate;

pub struct MeshPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub vertex_count: usize,
    vertex_binding_builder: BindingBuilder,
    fragment_binding_builder: BindingBuilder,
}

#[derive(Delegate)]
pub struct MeshGeometry {
    pub rotation: Expression,
    pub position: Expression,
    pub scale: Expression,
}

#[derive(Delegate)]
pub struct MeshMaterial {
    pub color: Expression,
}

impl MeshPipeline {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        geometry: &MeshGeometry,
        material: &MeshMaterial,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let &RenderingDescriptor {
            device,
            camera,
            format,
            ..
        } = rendering_descriptor;

        let vertex_size = size_of::<MeshVertexAttributes>();

        let mut module = naga::front::wgsl::parse_str(include_str!("../mesh.wgsl"))
            .unwrap_or_else(|_| panic!("{}", "Failed to parse {file_name}"));
        let info =
            naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
                .validate(&module)
                .unwrap_or_else(|_| panic!("{}", "Failed to validate {file_name}"));
        let pre_output_str = naga::back::wgsl::write_string(&module, &info, WriterFlags::all())
            .unwrap_or_else(|_| panic!("{}", "Failed to write new shader mased on {file_name}"));
        log::debug!("Original shader code:\n{pre_output_str}");
        log::debug!("Injecting instance");
        let mut vertex_binding_builder = BindingBuilder::new(&module, "vs_main", 1);
        geometry.inject("geometry", &mut module, &mut vertex_binding_builder);
        let mut fragment_binding_builder = BindingBuilder::new(&module, "fs_main", 0);
        material.inject("material", &mut module, &mut fragment_binding_builder);

        log::debug!("Validating generated mesh shader\n{module:#?}");
        let info =
            naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
                .validate(&module)
                .unwrap_or_else(|_| panic!("{}", "Failed to validate modified {file_name}"));
        let output_str = naga::back::wgsl::write_string(&module, &info, WriterFlags::all())
            .unwrap_or_else(|_| panic!("{}", "Failed to write new shader for {file_name}"));
        log::debug!("Resulting mesh shader code:\n{output_str}");

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mesh shader module"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&output_str)),
        });

        let vertex_uniform_bind_group_layouts: Vec<&BindGroupLayout> = vertex_binding_builder
            .uniforms
            .values()
            .map(|binding| binding.bind_group_layout.as_ref())
            .collect();

        let fragment_texture_bind_group_layouts: Vec<BindGroupLayout> = fragment_binding_builder
            .textures
            .values()
            .map(|binding| binding.inner.borrow().bind_group_layout.clone())
            .collect();

        let bind_group_layouts = {
            let mut layouts = vec![&camera.bind_group_layout];
            for layout in &vertex_uniform_bind_group_layouts {
                layouts.push(layout);
            }
            for layout in &fragment_texture_bind_group_layouts {
                layouts.push(layout);
            }
            layouts
        };

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh pipeline layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3, // position
                1 => Float32x3, // normal
                2 => Float32x2, // uv
            ],
        };

        let mut layouts = vertex_binding_builder
            .instances
            .values()
            .map(|binding| binding.layout.build())
            .collect();

        let buffers = {
            let mut buffers = vec![vertex_buffer_layout];
            buffers.append(&mut layouts);
            buffers
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh pipeline"),
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh vertex buffer"),
            contents: bytemuck::cast_slice(&Vec::<MeshVertexAttributes>::new()),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh index buffer"),
            contents: bytemuck::cast_slice(&Vec::<u32>::new()),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(MeshPipeline {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            vertex_count: 0,
            vertex_binding_builder,
            fragment_binding_builder,
        })
    }

    pub fn render(
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
        log::debug!("Rendering meshes");

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
        log::debug!("Mesh count {count:#?}");

        let instances: Vec<(&InstanceBinding, Ref<wgpu::Buffer>)> = self
            .vertex_binding_builder
            .instances
            .values()
            .map(|v| (v, Ref::map(v.inner.borrow(), |v| &v.buffer)))
            .collect();

        let uniforms: Vec<wgpu::BindGroup> = self
            .vertex_binding_builder
            .uniforms
            .values()
            .map(|v| v.inner.borrow().bind_group.clone())
            .collect();

        let textures: Vec<wgpu::BindGroup> = self
            .fragment_binding_builder
            .textures
            .values()
            .map(|v| v.inner.borrow().bind_group.clone())
            .collect();

        let mut render_pass = encoder.begin_render_pass(
            &DefaultRenderPassDescriptor::new(
                "meshes",
                view,
                multisampled_framebuffer,
                depth_texture,
            )
            .build(),
        );

        render_pass.set_bind_group(0, &camera.bind_group, &[]);

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        let mut instance_count = usize::from(count.is_none());
        for (binding, buffer) in instances.iter() {
            let slot = binding.slot;
            log::debug!("Setting vertex buffer {slot}");
            render_pass.set_vertex_buffer(slot, buffer.slice(..));
            instance_count = instance_count.max(binding.inner.borrow().count);
        }

        let mut current_slot = 1;

        for bind_group in textures.iter() {
            log::debug!("Setting texture bind group at slot {}", current_slot);
            render_pass.set_bind_group(current_slot, bind_group, &[]);
            current_slot += 1;
        }

        for bind_group in uniforms.iter() {
            log::debug!("Setting bind group {}", current_slot);
            render_pass.set_bind_group(current_slot, bind_group, &[]);
            current_slot += 1;
        }

        log::debug!("Drawing {instance_count} instances");
        render_pass.draw_indexed(0..self.vertex_count as u32, 0, 0..instance_count as u32);
    }
}
