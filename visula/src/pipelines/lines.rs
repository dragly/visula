use crate::{Application, BindingBuilder, InstanceBinding};
use bytemuck::{Pod, Zeroable};
use naga::{valid::ValidationFlags, Block, Handle, Statement};
use std::collections::HashMap;
use std::mem::size_of;
use visula_derive::define_delegate;
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

define_delegate! {
    pub struct LineDelegate {
        pub start: vec3,
        pub end: vec3,
        pub width: f32,
    }
}

impl Lines {
    pub fn new(
        application: &Application,
        delegate: &LineDelegate,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let Application {
            device,
            camera_bind_group_layout,
            ..
        } = application;
        let mut module =
            naga::front::wgsl::parse_str(include_str!("../shaders/line.wgsl")).unwrap();
        let entry_point_index = module
            .entry_points
            .iter()
            .position(|entry_point| entry_point.name == "vs_main")
            .unwrap();
        let mut binding_builder = BindingBuilder {
            bindings: HashMap::new(),
            uniforms: HashMap::new(),
            bind_groups: HashMap::new(),
            entry_point_index,
            shader_location_offset: 2,
            current_slot: 1,
            current_bind_group: 1,
        };

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

        let info =
            naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
                .validate(&module)
                .unwrap();
        let output_str = naga::back::wgsl::write_string(&module, &info).unwrap();
        log::debug!("Resulting lines shader code:\n{}", output_str);

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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline"),
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
            .iter()
            .map(|(_id, binding)| binding.layout.build())
            .collect();
        let buffers = {
            let mut buffers = vec![vertex_buffer_layout];
            buffers.append(&mut layouts);
            buffers
        };
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render pipeline"),
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
        });
        Ok(Lines {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
            binding_builder,
        })
    }

    pub fn render<'a>(
        &'a mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bindings: &[&'a dyn InstanceBinding<'a>],
    ) {
        log::debug!("Rendering lines");
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        let mut instance_count = 0;
        for binding in bindings {
            if self
                .binding_builder
                .bindings
                .contains_key(&binding.handle())
            {
                let slot = self.binding_builder.bindings[&binding.handle()].slot;
                log::debug!("Setting vertex buffer {}", slot);
                render_pass.set_vertex_buffer(slot, binding.buffer().slice(..));
                instance_count = instance_count.max(binding.count());
            }
            if self
                .binding_builder
                .uniforms
                .contains_key(&binding.handle())
            {
                log::debug!("Setting bind group {}", 1);
                render_pass.set_bind_group(1, binding.bind_group(), &[]);
            }
        }
        render_pass.draw_indexed(0..self.index_count as u32, 0, 0..instance_count);
    }
}
