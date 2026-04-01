use crate::pipelines::quad::{QuadPipeline, QuadPipelineDescriptor};
use crate::rendering_descriptor::RenderingDescriptor;
use crate::simulation::{RenderData, ShadowRenderData};
use crate::Renderable;
use bytemuck::{Pod, Zeroable};
use std::mem::size_of;
use visula_core::Expression;
use visula_derive::Delegate;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    _pos: [f32; 3],
    _tex_coord: [f32; 2],
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
    Vertex {
        _pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32],
        _tex_coord: [tc[0] as f32, tc[1] as f32],
    }
}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        vertex([-1, -1, -1], [0, 0]),
        vertex([1, -1, -1], [1, 0]),
        vertex([1, 1, -1], [1, 1]),
        vertex([-1, 1, -1], [0, 1]),
    ];
    let index_data: &[u16] = &[0, 1, 2, 2, 3, 0];
    (vertex_data.to_vec(), index_data.to_vec())
}

pub struct Spheres(QuadPipeline);

#[derive(Delegate)]
pub struct SphereGeometry {
    pub position: Expression,
    pub radius: Expression,
    pub color: Expression,
}

#[derive(Delegate)]
pub struct SphereMaterial {
    pub color: Expression,
}

impl Spheres {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        geometry: &SphereGeometry,
        material: &SphereMaterial,
    ) -> Result<Self, visula_core::ShaderError> {
        let (vertex_data, index_data) = create_vertices();
        Ok(Spheres(QuadPipeline::new(
            rendering_descriptor,
            &QuadPipelineDescriptor {
                label: "spheres",
                shader_source: include_str!("../shaders/sphere.wgsl"),
                shader_variable_name: "sphere_geometry",
                fragment_shader_variable_name: Some("sphere_material"),
                shadow_shader_source: Some(include_str!("../shaders/sphere_shadow.wgsl")),
                vertex_data: bytemuck::cast_slice(&vertex_data),
                vertex_stride: size_of::<Vertex>(),
                vertex_format: wgpu::VertexFormat::Float32x4,
                index_data: bytemuck::cast_slice(&index_data),
                index_format: wgpu::IndexFormat::Uint16,
            },
            geometry,
            Some(material),
        )?))
    }
}

impl Renderable for Spheres {
    fn render(&self, render_data: &mut RenderData) {
        self.0.render(render_data);
    }
    fn render_shadow(&self, shadow_data: &mut ShadowRenderData) {
        self.0.render_shadow(shadow_data);
    }
}
