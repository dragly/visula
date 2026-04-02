use glam::{Quat, Vec3};
use visula::{
    primitives::mesh_primitive::MeshVertexAttributes, Expression, MeshGeometry, MeshMaterial,
    MeshPipeline,
};
use wgpu::util::DeviceExt;

const PI: f32 = std::f32::consts::PI;

struct MeshExample {
    mesh: MeshPipeline,
}

fn trefoil_knot(t: f32) -> [f32; 3] {
    let x = (t).sin() + 2.0 * (2.0 * t).sin();
    let y = (t).cos() - 2.0 * (2.0 * t).cos();
    let z = -(3.0 * t).sin();
    [x, y, z]
}

impl MeshExample {
    pub fn new(app: &mut visula::Application) -> Self {
        let device = &app.device;

        let length_segments = 256u32;
        let radial_segments = 16u32;
        let tube_radius = 0.4;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for i in 0..=length_segments {
            let t = i as f32 / length_segments as f32 * 2.0 * PI;
            let dt = 0.001;
            let p = trefoil_knot(t);
            let p_next = trefoil_knot(t + dt);

            let tangent =
                Vec3::new(p_next[0] - p[0], p_next[1] - p[1], p_next[2] - p[2]).normalize();
            let up = if tangent.y.abs() < 0.99 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let normal_dir = tangent.cross(up).normalize();
            let binormal = tangent.cross(normal_dir).normalize();

            for j in 0..=radial_segments {
                let u = j as f32 / radial_segments as f32;
                let angle = u * 2.0 * PI;
                let circle_offset = normal_dir * angle.cos() + binormal * angle.sin();
                let pos = Vec3::new(p[0], p[1], p[2]) + circle_offset * tube_radius;
                let normal = circle_offset.normalize();

                let r = ((t * 3.0).sin() * 0.5 + 0.5) * 255.0;
                let g = ((t * 5.0 + 2.0).cos() * 0.5 + 0.5) * 255.0;
                let b = ((t * 7.0 + 4.0).sin() * 0.5 + 0.5) * 255.0;

                vertices.push(MeshVertexAttributes {
                    position: pos.into(),
                    normal: normal.into(),
                    uv: [t / (2.0 * PI), u],
                    color: [r as u8, g as u8, b as u8, 255],
                });
            }
        }

        for i in 0..length_segments {
            for j in 0..radial_segments {
                let a = i * (radial_segments + 1) + j;
                let b = a + radial_segments + 1;
                indices.push(a);
                indices.push(b);
                indices.push(a + 1);
                indices.push(b);
                indices.push(b + 1);
                indices.push(a + 1);
            }
        }

        let mut mesh = MeshPipeline::new(
            &app.rendering_descriptor(),
            &MeshGeometry {
                position: Vec3::ZERO.into(),
                rotation: Quat::IDENTITY.into(),
                scale: Vec3::ONE.into(),
            },
            &MeshMaterial {
                color: Expression::InputColor.lit(),
            },
        )
        .unwrap();

        mesh.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Trefoil vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        mesh.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Trefoil index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        mesh.vertex_count = indices.len();

        Self { mesh }
    }
}

impl visula::Simulation for MeshExample {
    type Error = ();

    fn render(&mut self, data: &mut visula::RenderData) {
        self.mesh.render(data);
    }
}

fn main() {
    visula::run(MeshExample::new);
}
