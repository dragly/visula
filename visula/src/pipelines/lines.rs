use crate::pipelines::quad::{QuadPipeline, QuadPipelineDescriptor};
use crate::rendering_descriptor::RenderingDescriptor;
use crate::simulation::RenderData;
use crate::Renderable;
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};
use std::mem::size_of;
use visula_core::Expression;
use visula_derive::Delegate;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    texture_coordinate: Vec2,
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

fn vertex(length_weight: f32, width_weight: f32) -> Vertex {
    Vertex {
        texture_coordinate: Vec2::new(length_weight, width_weight),
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

pub struct Lines(QuadPipeline);

#[derive(Delegate)]
pub struct LineDelegate {
    pub start: Expression,
    pub end: Expression,
    pub width: Expression,
    pub color: Expression,
}

impl Default for LineDelegate {
    fn default() -> Self {
        LineDelegate {
            start: Vec3::new(0.0, 0.0, 0.0).into(),
            end: Vec3::new(1.0, 0.0, 0.0).into(),
            width: 1.0.into(),
            color: Vec3::new(1.0, 1.0, 1.0).into(),
        }
    }
}

impl Lines {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        delegate: &LineDelegate,
    ) -> Result<Self, visula_core::ShaderError> {
        let (vertex_data, index_data) = create_vertices();
        Ok(Lines(QuadPipeline::new(
            rendering_descriptor,
            &QuadPipelineDescriptor {
                label: "lines",
                shader_source: include_str!("../shaders/line.wgsl"),
                shader_variable_name: "line_input",
                vertex_data: bytemuck::cast_slice(&vertex_data),
                vertex_stride: size_of::<Vertex>(),
                vertex_format: wgpu::VertexFormat::Float32x2,
                index_data: bytemuck::cast_slice(&index_data),
                index_format: wgpu::IndexFormat::Uint16,
            },
            delegate,
        )?))
    }
}

impl Renderable for Lines {
    fn render(&self, render_data: &mut RenderData) {
        self.0.render(render_data);
    }
}
