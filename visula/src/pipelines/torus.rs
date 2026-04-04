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
    position: [f32; 3],
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

fn create_box_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let v = |x: f32, y: f32, z: f32| Vertex {
        position: [x, y, z],
    };
    let vertex_data = vec![
        v(-1.0, -1.0, -1.0),
        v(1.0, -1.0, -1.0),
        v(1.0, 1.0, -1.0),
        v(-1.0, 1.0, -1.0),
        v(-1.0, -1.0, 1.0),
        v(1.0, -1.0, 1.0),
        v(1.0, 1.0, 1.0),
        v(-1.0, 1.0, 1.0),
    ];
    #[rustfmt::skip]
    let index_data: Vec<u16> = vec![
        0, 1, 2, 2, 3, 0,
        1, 5, 6, 6, 2, 1,
        5, 4, 7, 7, 6, 5,
        4, 0, 3, 3, 7, 4,
        3, 2, 6, 6, 7, 3,
        4, 5, 1, 1, 0, 4,
    ];
    (vertex_data, index_data)
}

pub struct Torus(QuadPipeline);

#[derive(Delegate)]
pub struct TorusGeometry {
    pub position: Expression,
    pub major_radius: Expression,
    pub minor_radius: Expression,
    pub rotation: Expression,
    pub color: Expression,
}

#[derive(Delegate)]
pub struct TorusMaterial {
    pub color: Expression,
}

impl Torus {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        geometry: &TorusGeometry,
        material: &TorusMaterial,
    ) -> Result<Self, visula_core::ShaderError> {
        let (vertex_data, index_data) = create_box_vertices();
        Ok(Torus(QuadPipeline::new(
            rendering_descriptor,
            &QuadPipelineDescriptor {
                label: "torus",
                shader_source: include_str!("../shaders/torus.wgsl"),
                shader_variable_name: "torus_geometry",
                fragment_shader_variable_name: Some("torus_material"),
                shadow_shader_source: Some(include_str!("../shaders/torus_shadow.wgsl")),
                vertex_data: bytemuck::cast_slice(&vertex_data),
                vertex_stride: size_of::<Vertex>(),
                vertex_format: wgpu::VertexFormat::Float32x3,
                index_data: bytemuck::cast_slice(&index_data),
                index_format: wgpu::IndexFormat::Uint16,
            },
            geometry,
            Some(material),
        )?))
    }
}

impl Renderable for Torus {
    fn render(&self, render_data: &mut RenderData) {
        self.0.render(render_data);
    }
    fn render_shadow(&self, shadow_data: &mut ShadowRenderData) {
        self.0.render_shadow(shadow_data);
    }
}
