use std::cell::Ref;
use std::mem::size_of;

use naga::back::wgsl::WriterFlags;
use naga::valid::ValidationFlags;
use wgpu::util::DeviceExt;

use crate::primitives::mesh_primitive::MeshVertexAttributes;
use crate::{DefaultRenderPassDescriptor, RenderData, RenderingDescriptor};
use visula_core::{BindingBuilder, BufferBinding, Expression};
use visula_derive::Delegate;

pub struct MeshPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub vertex_count: usize,
    binding_builder: BindingBuilder,
}

#[derive(Delegate)]
pub struct MeshDelegate {
    pub rotation: Expression,
    pub position: Expression,
}

impl MeshPipeline {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        delegate: &MeshDelegate,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let &RenderingDescriptor {
            device,
            camera,
            format,
            ..
        } = rendering_descriptor;
        let vertex_size = size_of::<MeshVertexAttributes>();
        let mut module = naga::front::wgsl::parse_str(include_str!("../mesh.wgsl")).unwrap();
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 1);

        delegate.inject("instance", &mut module, &mut binding_builder);
        log::debug!("Validating generated mesh shader\n{module:#?}");
        let info =
            naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
                .validate(&module)
                .unwrap();
        let output_str =
            naga::back::wgsl::write_string(&module, &info, WriterFlags::all()).unwrap();
        log::debug!("Resulting mesh shader code:\n{}", output_str);

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&output_str)),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh pipeline layout"),
            bind_group_layouts: &[&camera.bind_group_layout],
            push_constant_ranges: &[],
        });
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4],
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
            label: Some("Mesh pipeline"),
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: bytemuck::cast_slice(&Vec::<MeshVertexAttributes>::new()),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(&Vec::<u32>::new()),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(MeshPipeline {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            vertex_count: 0,
            binding_builder,
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
        log::debug!("Mesh count {count:#?}");
        let bindings: Vec<(&BufferBinding, Ref<wgpu::Buffer>)> = self
            .binding_builder
            .bindings
            .values()
            .map(|v| (v, Ref::map(v.inner.borrow(), |v| &v.buffer)))
            .collect();
        let uniforms: Vec<Ref<wgpu::BindGroup>> = self
            .binding_builder
            .uniforms
            .values()
            .map(|v| Ref::map(v.inner.borrow(), |m| &m.bind_group))
            .collect();
        {
            let default_render_pass = DefaultRenderPassDescriptor::new(
                "meshes",
                view,
                multisampled_framebuffer,
                depth_texture,
            );
            let mut render_pass = encoder.begin_render_pass(&default_render_pass.build());
            render_pass.set_bind_group(0, &camera.bind_group, &[]);

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            let mut instance_count = usize::from(count.is_none());
            for (binding, buffer) in bindings.iter() {
                let slot = binding.slot;
                log::debug!("Setting vertex buffer {}", slot);
                render_pass.set_vertex_buffer(slot, buffer.slice(..));
                instance_count = instance_count.max(binding.inner.borrow().count);
            }
            for bind_group in uniforms.iter() {
                log::debug!("Setting bind group {}", 1);
                render_pass.set_bind_group(1, bind_group, &[]);
            }
            log::debug!("Drawing {} instances", instance_count);
            render_pass.draw_indexed(0..self.vertex_count as u32, 0, 0..instance_count as u32);
        }
    }
}
