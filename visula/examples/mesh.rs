use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3, Vec4};
use visula::{
    primitives::{generate_sphere, mesh_primitive::MeshVertexAttributes},
    srgb, InstanceBuffer, MeshBlending, MeshGeometry, MeshMaterial, MeshPipeline,
};
use visula_derive::Instance;
use web_time::Instant;

const PI: f32 = std::f32::consts::PI;

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct Bubble {
    position: Vec3,
    _padding: f32,
    color: Vec4,
}

struct MeshExample {
    mesh: MeshPipeline,
    glass: MeshPipeline,
    bubbles: MeshPipeline,
    bubble_buffer: InstanceBuffer<Bubble>,
    start: Instant,
}

fn trefoil_knot(t: f32) -> [f32; 3] {
    let x = (t).sin() + 2.0 * (2.0 * t).sin();
    let y = (t).cos() - 2.0 * (2.0 * t).cos();
    let z = -(3.0 * t).sin();
    [x, y, z]
}

fn bubbles(seconds: f32) -> Vec<Bubble> {
    let rings = [
        (
            Quat::from_rotation_x(0.5),
            1.0,
            [0xE63946, 0xF4A261, 0xE9C46A, 0xF15BB5, 0x9B5DE5],
        ),
        (
            Quat::from_rotation_x(-0.5),
            -1.0,
            [0x00BBF9, 0x00F5D4, 0x57CC99, 0x4361EE, 0x7209B7],
        ),
    ];
    rings
        .iter()
        .flat_map(|(tilt, direction, colors)| {
            colors.iter().enumerate().map(move |(index, color)| {
                let angle =
                    direction * 0.5 * seconds + index as f32 / colors.len() as f32 * 2.0 * PI;
                Bubble {
                    position: *tilt * Vec3::new(angle.cos(), 0.0, angle.sin()) * 4.0,
                    _padding: 0.0,
                    color: srgb(*color).extend(0.5),
                }
            })
        })
        .collect()
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
            &MeshGeometry::default(),
            &MeshMaterial::default(),
        )
        .unwrap();

        mesh.set_mesh_data(device, &vertices, &indices);

        let mut glass = MeshPipeline::with_blending(
            &app.rendering_descriptor(),
            &MeshGeometry {
                color: Vec4::new(0.6, 0.8, 1.0, 0.3).into(),
                ..Default::default()
            },
            &MeshMaterial::default(),
            MeshBlending::Transparent,
        )
        .unwrap();

        let (glass_vertices, glass_indices) = generate_sphere(4.0, 64, 32, [255, 255, 255, 255]);
        glass.set_mesh_data(device, &glass_vertices, &glass_indices);

        let bubble_buffer = InstanceBuffer::<Bubble>::new(device);
        let bubble = bubble_buffer.instance();
        let mut bubbles = MeshPipeline::with_blending(
            &app.rendering_descriptor(),
            &MeshGeometry {
                position: bubble.position,
                color: bubble.color,
                ..Default::default()
            },
            &MeshMaterial::default(),
            MeshBlending::Transparent,
        )
        .unwrap();

        let (bubble_vertices, bubble_indices) = generate_sphere(1.0, 32, 16, [255, 255, 255, 255]);
        bubbles.set_mesh_data(device, &bubble_vertices, &bubble_indices);

        app.camera_controller.current_transform.distance = 16.0;
        app.camera_controller.target_transform = app.camera_controller.current_transform.clone();

        Self {
            mesh,
            glass,
            bubbles,
            bubble_buffer,
            start: Instant::now(),
        }
    }
}

impl visula::Simulation for MeshExample {
    fn update(&mut self, app: &mut visula::Application) {
        let seconds = self.start.elapsed().as_secs_f32();
        self.bubble_buffer
            .update(&app.device, &app.queue, &bubbles(seconds));
    }

    fn render(&mut self, data: &mut visula::RenderData) {
        self.mesh.render(data);
        self.glass.render(data);
        self.bubbles.render(data);
    }
}

fn main() {
    visula::run(MeshExample::new);
}
