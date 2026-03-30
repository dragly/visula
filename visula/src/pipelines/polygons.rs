use crate::pipelines::quad::{QuadPipeline, QuadPipelineDescriptor};
use crate::rendering_descriptor::RenderingDescriptor;
use crate::simulation::RenderData;
use crate::Renderable;
use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};
use std::mem::size_of;
use visula_core::Expression;
use visula_derive::Delegate;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PolygonVertex {
    pub position: [f32; 2],
}

pub struct Polygons(QuadPipeline);

#[derive(Delegate)]
pub struct PolygonDelegate {
    pub color: Expression,
    pub position: Expression,
}

impl Default for PolygonDelegate {
    fn default() -> Self {
        PolygonDelegate {
            color: Vec4::new(1.0, 1.0, 1.0, 1.0).into(),
            position: Vec3::ZERO.into(),
        }
    }
}

impl Polygons {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        delegate: &PolygonDelegate,
        vertices: &[PolygonVertex],
        indices: &[u32],
    ) -> Result<Self, visula_core::ShaderError> {
        Ok(Polygons(QuadPipeline::new(
            rendering_descriptor,
            &QuadPipelineDescriptor {
                label: "polygons",
                shader_source: include_str!("../shaders/polygon.wgsl"),
                shader_variable_name: "polygon",
                fragment_shader_variable_name: None,
                vertex_data: bytemuck::cast_slice(vertices),
                vertex_stride: size_of::<PolygonVertex>(),
                vertex_format: wgpu::VertexFormat::Float32x2,
                index_data: bytemuck::cast_slice(indices),
                index_format: wgpu::IndexFormat::Uint32,
            },
            delegate,
            None,
        )?))
    }
}

impl Renderable for Polygons {
    fn render(&self, render_data: &mut RenderData) {
        self.0.render(render_data);
    }
}
