use glam::{Quat, Vec3};
use visula::{
    primitives::mesh_primitive::MeshVertexAttributes, Expression, MeshGeometry, MeshMaterial,
    MeshPipeline,
};
use visula_core::TextureBuffer;
use wgpu::util::DeviceExt;

struct PlaneExample {
    mesh: MeshPipeline,
}

impl PlaneExample {
    pub fn new(app: &mut visula::Application) -> Self {
        let device = &app.device;

        let size = wgpu::Extent3d {
            width: 640,
            height: 480,
            depth_or_array_layers: 1,
        };
        let texture = TextureBuffer::<f32>::new(device, size);

        let mut mesh = MeshPipeline::new(
            &app.rendering_descriptor(),
            &MeshGeometry {
                position: Vec3::ZERO.into(),
                rotation: Quat::IDENTITY.into(),
                scale: (100.0 * Vec3::ONE).into(),
            },
            &MeshMaterial {
                color: texture.sample(&Expression::UV),
            },
        )
        .unwrap();

        let vertices: Vec<MeshVertexAttributes> = vec![
            MeshVertexAttributes {
                position: [-0.5, -0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0, 0.0],
                color: [255, 255, 255, 255],
            },
            MeshVertexAttributes {
                position: [0.5, -0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [1.0, 0.0],
                color: [255, 255, 255, 255],
            },
            MeshVertexAttributes {
                position: [0.5, 0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [1.0, 1.0],
                color: [255, 255, 255, 255],
            },
            MeshVertexAttributes {
                position: [-0.5, 0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0, 1.0],
                color: [255, 255, 255, 255],
            },
        ];

        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        mesh.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Textured plane vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        mesh.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Textured plane index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        mesh.vertex_count = indices.len();

        let width: u32 = size.width;
        let height: u32 = size.height;
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        let checker_size: u32 = 32;

        for y in 0..height {
            for x in 0..width {
                let cx = (x / checker_size) % 2;
                let cy = (y / checker_size) % 2;
                let v = if (cx + cy).is_multiple_of(2) { 230 } else { 30 };
                let idx = ((y * width + x) * 4) as usize;
                rgba[idx] = v; // R
                rgba[idx + 1] = v; // G
                rgba[idx + 2] = v; // B
                rgba[idx + 3] = 255; // A
            }
        }

        texture.update(device, &app.queue, size, &rgba);

        Self { mesh }
    }
}

impl visula::Simulation for PlaneExample {
    type Error = ();

    fn render(&mut self, data: &mut visula::RenderData) {
        self.mesh.render(data);
    }
}

fn main() {
    visula::run(PlaneExample::new);
}
