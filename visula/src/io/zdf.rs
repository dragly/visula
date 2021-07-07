use cgmath::EuclideanSpace;
use ndarray::s;
use std::path::Path;
use wgpu::util::DeviceExt;

use crate::primitives::mesh::MeshVertexAttributes;
use crate::primitives::sphere::Sphere;
use crate::Point3;
use crate::Vector3;

pub struct ZdfFile {
    pub camera_center: Vector3,
    pub instance_buffer: wgpu::Buffer,
    pub instance_count: usize,
    pub mesh_vertex_buf: wgpu::Buffer,
    pub mesh_vertex_count: usize,
}

pub fn read_zdf(path: &Path, device: &mut wgpu::Device) -> ZdfFile {
    let name: &str = path.to_str().unwrap();
    let file = netcdf::open(name).unwrap();
    let group = &file.group("data").unwrap().unwrap();
    let pointcloud = &group
        .variable("pointcloud")
        .expect("Could not find pointcloud");
    let rgba_image = &group
        .variable("rgba_image")
        .expect("Could not find pointcloud");

    let mut vertices = vec![];
    let points = pointcloud.values::<f32>(None, None).unwrap();
    let colors = rgba_image.values::<f32>(None, None).unwrap();
    for col in 0..(points.shape()[0] - 1) {
        for row in 0..(points.shape()[1] - 1) {
            let col_m = (col as i64 - 1).max(0) as usize;
            let row_m = (row as i64 - 1).max(0) as usize;
            let col_p = (col as i64 + 1).min(points.shape()[0] as i64 - 1) as usize;
            let row_p = (row as i64 + 1).min(points.shape()[1] as i64 - 1) as usize;

            let color = colors.slice(s![col, row, ..]);
            let color = [color[0] as u8, color[1] as u8, color[2] as u8, 255];
            let point_c = points.slice(s![col, row, ..]);
            let point_l = points.slice(s![col_m, row, ..]);
            let point_r = points.slice(s![col_p, row, ..]);
            let point_t = points.slice(s![col, row_m, ..]);
            let point_b = points.slice(s![col, row_p, ..]);
            let point_tl = points.slice(s![col_m, row_m, ..]);
            let point_bl = points.slice(s![col_m, row_p, ..]);
            let point_tr = points.slice(s![col_p, row_m, ..]);
            let point_br = points.slice(s![col_p, row_p, ..]);

            let corner_tr = (&point_r + &point_tr + point_c + point_t) / 4.0;
            let corner_tl = (&point_l + &point_tl + point_c + point_t) / 4.0;
            let corner_br = (&point_r + &point_br + point_c + point_b) / 4.0;
            let corner_bl = (&point_l + &point_bl + point_c + point_b) / 4.0;
            if !corner_tr[0].is_nan()
                && !corner_tl[0].is_nan()
                && !corner_br[0].is_nan()
                && !corner_bl[0].is_nan()
            {
                vertices.push(MeshVertexAttributes {
                    position: [corner_tr[0], corner_tr[1], corner_tr[2]],
                    color,
                    normal: [1.0, 0.0, 0.0],
                });
                vertices.push(MeshVertexAttributes {
                    position: [corner_tl[0], corner_tl[1], corner_tl[2]],
                    color,
                    normal: [1.0, 0.0, 0.0],
                });
                vertices.push(MeshVertexAttributes {
                    position: [corner_br[0], corner_br[1], corner_br[2]],
                    color,
                    normal: [1.0, 0.0, 0.0],
                });
                vertices.push(MeshVertexAttributes {
                    position: [corner_tl[0], corner_tl[1], corner_tl[2]],
                    color,
                    normal: [1.0, 0.0, 0.0],
                });
                vertices.push(MeshVertexAttributes {
                    position: [corner_bl[0], corner_bl[1], corner_bl[2]],
                    color,
                    normal: [1.0, 0.0, 0.0],
                });
                vertices.push(MeshVertexAttributes {
                    position: [corner_br[0], corner_br[1], corner_br[2]],
                    color,
                    normal: [1.0, 0.0, 0.0],
                });
            }
        }
    }

    let mut mean_position = Point3::new(0.0, 0.0, 0.0);
    assert!(points.shape()[2] == 3);
    let points_shape = (points.shape()[0] * points.shape()[1], points.shape()[2]);
    let colors_shape = (colors.shape()[0] * colors.shape()[1], colors.shape()[2]);
    let points_flat = points.into_shape(points_shape).unwrap();
    let colors_flat = colors.into_shape(colors_shape).unwrap();
    let instance_data: Vec<Sphere> = points_flat
        .outer_iter()
        .zip(colors_flat.outer_iter())
        .filter_map(|(point, color)| {
            let x = point[0];
            let y = point[1];
            let z = point[2];
            if x.is_nan() || y.is_nan() || z.is_nan() {
                return None;
            }
            let position = Point3::new(x, y, z);
            let color = Point3::new(color[0] / 255.0, color[1] / 255.0, color[2] / 255.0);
            let radius = 1.0;

            mean_position += position.to_vec();

            Some(Sphere {
                position: position.into(),
                radius,
                color: color.into(),
            })
        })
        .collect();
    let instance_count = instance_data.len();

    let camera_center = (mean_position / instance_count as f32).to_vec();

    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance buffer"),
        contents: bytemuck::cast_slice(&instance_data),
        usage: wgpu::BufferUsage::VERTEX,
    });

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Mesh buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsage::VERTEX,
    });

    ZdfFile {
        instance_buffer,
        instance_count,
        mesh_vertex_buf: vertex_buffer,
        mesh_vertex_count: vertices.len(),
        camera_center,
    }
}
