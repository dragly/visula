use std::io::Read;
use std::io::Seek;
use wgpu::util::DeviceExt;

use crate::error::Error;
use crate::primitives::mesh_primitive::MeshVertexAttributes;
use crate::Application;

pub struct GltfFile {
    pub scenes: Vec<GltfScene>,
}

pub struct GltfScene {
    pub meshes: Vec<GltfMesh>,
}

pub struct GltfMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: usize,
}

pub fn import_buffer_data(
    document: &gltf::Document,
    mut blob: Option<Vec<u8>>,
) -> Result<Vec<Vec<u8>>, Error> {
    let mut buffers = Vec::new();
    for buffer in document.buffers() {
        let mut data = match buffer.source() {
            gltf::buffer::Source::Uri(_) => return Err(Error::GltfMissingBlobData),
            gltf::buffer::Source::Bin => blob.take().ok_or(Error::GltfMissingBlobData)?,
        };
        assert!(
            data.len() >= buffer.length(),
            "Data length is less than buffer length"
        );
        while data.len() % 4 != 0 {
            data.push(0);
        }
        buffers.push(data);
    }
    Ok(buffers)
}

pub fn parse_gltf(
    reader: &mut (impl Read + Seek),
    application: &Application,
) -> Result<GltfFile, Error> {
    let file = gltf::Gltf::from_reader(reader)?;
    let document = file.document;
    let blob = file.blob;
    let buffers = import_buffer_data(&document, blob)?;

    fn collect_meshes(
        node: gltf::Node,
        buffers: &[Vec<u8>],
        application: &Application,
        meshes: &mut Vec<GltfMesh>,
    ) {
        if let Some(mesh) = node.mesh() {
            let mut indices = vec![];
            let mut vertices = vec![];
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                if let Some(positions) = reader.read_positions() {
                    if let Some(normals) = reader.read_normals() {
                        for (position, normal) in positions.zip(normals) {
                            let color = [230, 100, 230, 255];
                            vertices.push(MeshVertexAttributes {
                                position,
                                normal,
                                uv: [0.0, 0.0],
                                color,
                            });
                        }
                    }
                }
                if let Some(indexes) = reader.read_indices() {
                    match indexes {
                        gltf::mesh::util::ReadIndices::U8(iter) => {
                            for index in iter {
                                indices.push(index as u32);
                            }
                        }
                        gltf::mesh::util::ReadIndices::U16(iter) => {
                            for index in iter {
                                indices.push(index as u32);
                            }
                        }
                        gltf::mesh::util::ReadIndices::U32(iter) => {
                            for index in iter {
                                indices.push(index);
                            }
                        }
                    }
                }
            }
            let index_buffer =
                application
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Index buffer"),
                        contents: bytemuck::cast_slice(&indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });
            let vertex_buffer =
                application
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Mesh buffer"),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
            meshes.push(GltfMesh {
                index_buffer,
                vertex_buffer,
                index_count: indices.len(),
            });
        }
        for child in node.children() {
            collect_meshes(child, buffers, application, meshes);
        }
    }

    let mut scenes = vec![];
    for scene in document.scenes() {
        let mut meshes = vec![];
        for node in scene.nodes() {
            collect_meshes(node, &buffers, application, &mut meshes);
        }
        scenes.push(GltfScene { meshes });
    }
    Ok(GltfFile { scenes })
}
