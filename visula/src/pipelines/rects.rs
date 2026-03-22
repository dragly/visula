use crate::pipelines::quad::{QuadPipeline, QuadPipelineDescriptor};
use crate::rendering_descriptor::RenderingDescriptor;
use crate::simulation::RenderData;
use crate::Renderable;
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3, Vec4};
use std::mem::size_of;
use visula_core::Expression;
use visula_derive::Delegate;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 2],
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        Vertex {
            position: [-1.0, -1.0],
        },
        Vertex {
            position: [1.0, -1.0],
        },
        Vertex {
            position: [1.0, 1.0],
        },
        Vertex {
            position: [-1.0, 1.0],
        },
    ];
    let index_data: &[u16] = &[0, 1, 2, 2, 3, 0];
    (vertex_data.to_vec(), index_data.to_vec())
}

pub struct Rects(QuadPipeline);

#[derive(Delegate)]
pub struct RectDelegate {
    pub position: Expression,
    pub size: Expression,
    pub corner_radius: Expression,
    pub fill_color: Expression,
    pub stroke_color: Expression,
    pub stroke_width: Expression,
}

impl Default for RectDelegate {
    fn default() -> Self {
        RectDelegate {
            position: Vec3::new(0.0, 0.0, 0.0).into(),
            size: Vec2::new(2.0, 2.0).into(),
            corner_radius: 0.0.into(),
            fill_color: Vec4::new(1.0, 1.0, 1.0, 1.0).into(),
            stroke_color: Vec4::new(0.0, 0.0, 0.0, 0.0).into(),
            stroke_width: 0.0.into(),
        }
    }
}

impl Rects {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        delegate: &RectDelegate,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (vertex_data, index_data) = create_vertices();
        Ok(Rects(QuadPipeline::new(
            rendering_descriptor,
            &QuadPipelineDescriptor {
                label: "rects",
                shader_source: include_str!("../shaders/rect.wgsl"),
                shader_variable_name: "rect",
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

impl Renderable for Rects {
    fn render(&self, render_data: &mut RenderData) {
        self.0.render(render_data);
    }
}
