use std::mem::size_of;

use wgpu::util::DeviceExt;

use crate::pipelines::pipeline::Pipeline;
use crate::primitives::mesh::MeshVertexAttributes;
use crate::{DefaultRenderPassDescriptor, RenderData};

pub struct MeshPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buf: wgpu::Buffer,
    pub index_buf: wgpu::Buffer,
    pub vertex_count: usize,
}

impl Pipeline for MeshPipeline {
    fn render(&mut self, data: &mut RenderData) {
        let RenderData {
            encoder,
            view,
            depth_texture,
            camera_bind_group,
            ..
        } = data;
        let default_render_pass = DefaultRenderPassDescriptor::new("mesh", view, depth_texture);
        let mut render_pass = encoder.begin_render_pass(&default_render_pass.build());
        render_pass.set_bind_group(0, camera_bind_group, &[]);

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        render_pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.vertex_count as u32, 0, 0..1);
    }
}

pub fn create_mesh_pipeline(
    application: &crate::Application,
) -> Result<MeshPipeline, Box<dyn std::error::Error>> {
    let crate::Application {
        device,
        camera,
        ..
    } = application;
    let vertex_size = size_of::<MeshVertexAttributes>();
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("../mesh.wgsl"))),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Mesh pipeline layout"),
        bind_group_layouts: &[&camera.bind_group_layout],
        push_constant_ranges: &[],
    });
    let buffer_layout = wgpu::VertexBufferLayout {
        array_stride: vertex_size as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4],
    };
    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Mesh pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[buffer_layout],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(application.config.format.into())],
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

    Ok(MeshPipeline {
        render_pipeline,
        vertex_buf,
        index_buf,
        vertex_count: 0,
    })
}
