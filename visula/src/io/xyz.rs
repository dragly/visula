use crate::primitives::sphere::Sphere;
use crate::Point3;
use wgpu::util::DeviceExt;

pub struct XyzFile {
    pub instance_buffer: wgpu::Buffer,
    pub instance_count: usize,
}

pub fn read_xyz(text: &[u8], device: &mut wgpu::Device) -> XyzFile {
    let text_bytes = text;
    let inner = std::io::Cursor::new(text_bytes);
    let mut trajectory = trajan::xyz::XYZReader::<f32, std::io::Cursor<&[u8]>>::new(
        trajan::coordinate::CoordKind::Position,
        inner,
    );
    let snapshot = trajectory.read_snapshot().unwrap();

    let instance_data: Vec<Sphere> = snapshot
        .particles
        .iter()
        .filter_map(|particle| {
            let position = match particle.xyz {
                trajan::coordinate::Coordinate::Position { x, y, z } => Some(Point3::new(x, y, z)),
                _ => None,
            };
            println!("Name {}", particle.name);
            let (color, radius) = match particle.name.as_ref() {
                "1" => (Point3::new(1.0, 0.0, 0.0), 1.0),
                "2" => (Point3::new(0.0, 1.0, 0.0), 0.4),
                _ => (Point3::new(1.0, 1.0, 1.0), 0.6),
            };
            position.map(|p| Sphere {
                position: p.into(),
                radius,
                color: color.into(),
            })
        })
        .collect();

    let instance_count = instance_data.len();

    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance buffer"),
        contents: bytemuck::cast_slice(&instance_data),
        usage: wgpu::BufferUsages::VERTEX,
    });

    XyzFile {
        instance_buffer,
        instance_count,
    }
}
