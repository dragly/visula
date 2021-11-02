use crate::primitives::sphere::Sphere;
use crate::Point3;

pub struct XyzFile {
    pub point_cloud: Vec<Sphere>,
}

pub fn read_xyz(text: &[u8], _device: &mut wgpu::Device) -> XyzFile {
    let text_bytes = text;
    let inner = std::io::Cursor::new(text_bytes);
    let mut trajectory = trajan::xyz::XYZReader::<f32, std::io::Cursor<&[u8]>>::new(
        trajan::coordinate::CoordKind::Position,
        inner,
    );
    let snapshot = trajectory.read_snapshot().unwrap();

    let point_cloud: Vec<Sphere> = snapshot
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
                padding: 0.0,
            })
        })
        .collect();

    XyzFile { point_cloud }
}
