use super::MeshVertexAttributes;

pub fn generate_plane(
    width: f32,
    height: f32,
    color: [u8; 4],
) -> (Vec<MeshVertexAttributes>, Vec<u32>) {
    let hw = width / 2.0;
    let hh = height / 2.0;
    let normal = [0.0, 1.0, 0.0];

    let vertices = vec![
        MeshVertexAttributes {
            position: [-hw, 0.0, -hh],
            normal,
            uv: [0.0, 0.0],
            color,
        },
        MeshVertexAttributes {
            position: [hw, 0.0, -hh],
            normal,
            uv: [1.0, 0.0],
            color,
        },
        MeshVertexAttributes {
            position: [hw, 0.0, hh],
            normal,
            uv: [1.0, 1.0],
            color,
        },
        MeshVertexAttributes {
            position: [-hw, 0.0, hh],
            normal,
            uv: [0.0, 1.0],
            color,
        },
    ];

    let indices = vec![0, 1, 2, 2, 3, 0];
    (vertices, indices)
}

pub fn generate_torus(
    major_radius: f32,
    minor_radius: f32,
    major_segments: usize,
    minor_segments: usize,
    color: [u8; 4],
) -> (Vec<MeshVertexAttributes>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=major_segments {
        let u = i as f32 / major_segments as f32;
        let theta = u * 2.0 * std::f32::consts::PI;
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        for j in 0..=minor_segments {
            let v = j as f32 / minor_segments as f32;
            let phi = v * 2.0 * std::f32::consts::PI;
            let cos_phi = phi.cos();
            let sin_phi = phi.sin();

            let x = (major_radius + minor_radius * cos_phi) * cos_theta;
            let y = minor_radius * sin_phi;
            let z = (major_radius + minor_radius * cos_phi) * sin_theta;

            let nx = cos_phi * cos_theta;
            let ny = sin_phi;
            let nz = cos_phi * sin_theta;

            vertices.push(MeshVertexAttributes {
                position: [x, y, z],
                normal: [nx, ny, nz],
                uv: [u, v],
                color,
            });
        }
    }

    for i in 0..major_segments {
        for j in 0..minor_segments {
            let a = i * (minor_segments + 1) + j;
            let b = a + minor_segments + 1;
            indices.push(a as u32);
            indices.push(b as u32);
            indices.push((a + 1) as u32);
            indices.push(b as u32);
            indices.push((b + 1) as u32);
            indices.push((a + 1) as u32);
        }
    }

    (vertices, indices)
}

pub fn generate_sphere(
    radius: f32,
    segments: usize,
    rings: usize,
    color: [u8; 4],
) -> (Vec<MeshVertexAttributes>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=rings {
        let v = i as f32 / rings as f32;
        let phi = v * std::f32::consts::PI;

        for j in 0..=segments {
            let u = j as f32 / segments as f32;
            let theta = u * 2.0 * std::f32::consts::PI;

            let x = phi.sin() * theta.cos();
            let y = phi.cos();
            let z = phi.sin() * theta.sin();

            vertices.push(MeshVertexAttributes {
                position: [x * radius, y * radius, z * radius],
                normal: [x, y, z],
                uv: [u, v],
                color,
            });
        }
    }

    for i in 0..rings {
        for j in 0..segments {
            let a = i * (segments + 1) + j;
            let b = a + segments + 1;
            indices.push(a as u32);
            indices.push(b as u32);
            indices.push((a + 1) as u32);
            indices.push(b as u32);
            indices.push((b + 1) as u32);
            indices.push((a + 1) as u32);
        }
    }

    (vertices, indices)
}
