use std::cell::Ref;
use std::mem::size_of;

use naga::back::wgsl::WriterFlags;
use naga::valid::ValidationFlags;
use visula_derive::define_delegate;
use wgpu::util::DeviceExt;
use wgpu::BindGroupLayout;

use crate::primitives::mesh::MeshVertexAttributes;
use crate::{BindingBuilder, BufferBinding, DefaultRenderPassDescriptor, SimulationRenderData};

pub struct Mesh {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buf: wgpu::Buffer,
    pub index_buf: wgpu::Buffer,
    pub vertex_count: usize,
    pub index_count: usize,
    binding_builder: BindingBuilder,
}

define_delegate! {
    pub struct MeshDelegate {
        pub position: vec3,
        pub scale: vec3,
    }
}

impl Mesh {
    pub fn new(
        application: &crate::Application,
        delegate: &MeshDelegate,
    ) -> Result<Mesh, Box<dyn std::error::Error>> {
        let crate::Application {
            device,
            camera_bind_group_layout,
            ..
        } = application;
        let vertex_size = size_of::<MeshVertexAttributes>();
        let mut module =
            naga::front::wgsl::parse_str(include_str!("../shaders/mesh.wgsl")).unwrap();
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 1);
        delegate.inject("mesh", &mut module, &mut binding_builder);
        log::debug!("Validating generated mesh shader");
        let info =
            naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
                .validate(&module)
                .unwrap();
        let output_str =
            naga::back::wgsl::write_string(&module, &info, WriterFlags::all()).unwrap();
        log::debug!("Resulting mesh shader code:\n{}", output_str);
        let shader_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&output_str)),
        });
        let bind_group_layouts: Vec<&BindGroupLayout> = binding_builder
            .uniforms
            .iter()
            .map(|(_id, binding)| binding.bind_group_layout.as_ref())
            .collect();

        let uniforms = {
            let mut uniforms = vec![camera_bind_group_layout];
            for layout in &bind_group_layouts {
                uniforms.push(layout);
            }
            uniforms
        };
        println!("Uniforms {}", uniforms.len());
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh pipeline layout"),
            bind_group_layouts: &uniforms,
            push_constant_ranges: &[],
        });
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4],
        };
        let mut layouts = binding_builder
            .bindings
            .iter()
            .map(|(_id, binding)| binding.layout.build())
            .collect();
        let buffers = {
            let mut buffers = vec![vertex_buffer_layout];
            buffers.append(&mut layouts);
            buffers
        };
        println!("Buffers {}", buffers.len());
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
            multiview: None,
        });

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: bytemuck::cast_slice(&Vec::<MeshVertexAttributes>::new()),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(&Vec::<u32>::new()),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Mesh {
            render_pipeline,
            vertex_buf,
            index_buf,
            vertex_count: 0,
            index_count: 0,
            binding_builder,
        })
    }
    pub fn render(&mut self, data: &mut SimulationRenderData) {
        let SimulationRenderData {
            encoder,
            view,
            depth_texture,
            camera_bind_group,
            ..
        } = data;

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
        log::debug!("Line count {count:#?}");
        if count.is_none() {
            log::debug!("Empty spheres buffer detected. Aborting render of spheres.");
            return;
        }
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
            let default_render_pass = DefaultRenderPassDescriptor::new("mesh", view, depth_texture);
            let mut render_pass = encoder.begin_render_pass(&default_render_pass.build());
            render_pass.set_bind_group(0, camera_bind_group, &[]);

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
            render_pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);
            let mut instance_count = 0;
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
            render_pass.draw_indexed(0..self.index_count as u32, 0, 0..instance_count as u32);
        }
    }
}
