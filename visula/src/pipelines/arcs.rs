use crate::pipelines::quad::{QuadPipeline, QuadPipelineDescriptor};
use crate::rendering_descriptor::RenderingDescriptor;
use crate::simulation::RenderData;
use crate::Renderable;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use std::mem::size_of;
use visula_core::Expression;
use visula_derive::Delegate;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    corner: [f32; 2],
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        Vertex {
            corner: [-1.0, -1.0],
        },
        Vertex {
            corner: [1.0, -1.0],
        },
        Vertex { corner: [1.0, 1.0] },
        Vertex {
            corner: [-1.0, 1.0],
        },
    ];
    let index_data: &[u16] = &[0, 1, 2, 2, 3, 0];
    (vertex_data.to_vec(), index_data.to_vec())
}

/// Flat strokes along circular arcs, with edges smoothed over one pixel at any zoom.
///
/// Each instance runs from `start` to `end` in the plane through both points with the given
/// `normal`. Its direction turns by `turn` radians on the way, counterclockwise around `normal`
/// when positive, so a `turn` of zero draws a straight stroke. Strokes end square at `start`
/// and `end`, so pieces that meet with the same direction join without a seam.
pub struct Arcs(QuadPipeline);

#[derive(Delegate)]
pub struct ArcGeometry {
    pub start: Expression,
    pub end: Expression,
    pub turn: Expression,
    pub width: Expression,
    pub normal: Expression,
    pub color: Expression,
}

#[derive(Delegate)]
pub struct ArcMaterial {
    pub color: Expression,
}

impl Default for ArcGeometry {
    fn default() -> Self {
        ArcGeometry {
            start: Vec3::new(0.0, 0.0, 0.0).into(),
            end: Vec3::new(1.0, 0.0, 0.0).into(),
            turn: 0.0.into(),
            width: 1.0.into(),
            normal: Vec3::new(0.0, 0.0, 1.0).into(),
            color: Vec3::new(1.0, 1.0, 1.0).into(),
        }
    }
}

impl Default for ArcMaterial {
    fn default() -> Self {
        ArcMaterial {
            color: Expression::InputColor.lit(),
        }
    }
}

impl Arcs {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        geometry: &ArcGeometry,
        material: &ArcMaterial,
    ) -> Result<Self, visula_core::ShaderError> {
        let (vertex_data, index_data) = create_vertices();
        Ok(Arcs(QuadPipeline::new(
            rendering_descriptor,
            &QuadPipelineDescriptor {
                label: "arcs",
                shader_source: include_str!("../shaders/arc.wgsl"),
                shader_variable_name: "arc_geometry",
                fragment_shader_variable_name: Some("arc_material"),
                shadow_shader_source: None,
                vertex_data: bytemuck::cast_slice(&vertex_data),
                vertex_stride: size_of::<Vertex>(),
                vertex_format: wgpu::VertexFormat::Float32x2,
                index_data: bytemuck::cast_slice(&index_data),
                index_format: wgpu::IndexFormat::Uint16,
                bind_group_layouts: &[],
                depth_write_enabled: true,
                depth_bias: wgpu::DepthBiasState::default(),
            },
            geometry,
            Some(material),
        )?))
    }
}

impl Renderable for Arcs {
    fn render(&self, render_data: &mut RenderData) {
        self.0.render(render_data);
    }
}
