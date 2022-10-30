use bytemuck::Pod;
use bytemuck::Zeroable;
use visula_derive::Instance;
use visula_derive::Uniform;
use std::io::Read;
use std::io::Seek;
use wgpu::util::DeviceExt;

use crate::error::Error;
use crate::primitives::mesh::MeshVertexAttributes;
use crate::{Application, Buffer};

impl From<gltf::Error> for Error {
    fn from(_: gltf::Error) -> Self {
        Error {}
    }
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error {}
    }
}

pub struct GltfFile {
    pub scenes: Vec<GltfScene>,
    pub animations: Vec<GltfAnimation>,
}

pub struct GltfScene {
    pub meshes: Vec<GltfMesh>,
}

pub struct GltfMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: usize,
    pub index_buffer: wgpu::Buffer,
    pub index_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Instance, Uniform, Zeroable)]
pub struct Scale {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct GltfChannel {
    pub target: usize,
    pub property: gltf::animation::Property,
    pub input_buffer: Buffer<f32>,
    pub output_buffer: Buffer<Scale>,
}

pub struct GltfAnimation {
    pub channels: Vec<GltfChannel>,
}

pub fn import_buffer_data(document: &gltf::Document, mut blob: Option<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut buffers = Vec::new();
    for buffer in document.buffers() {
        let mut data = match buffer.source() {
            gltf::buffer::Source::Uri(_) => panic!("Unsupported URI buffer"),
            gltf::buffer::Source::Bin => blob.take().unwrap(),
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
    buffers
}

pub fn parse_gltf(
    reader: &mut (impl Read + Seek),
    application: &mut Application,
) -> Result<GltfFile, Error> {
    let file = gltf::Gltf::from_reader(reader)?;
    let document = file.document;
    let blob = file.blob;
    let buffers = import_buffer_data(&document, blob);

    let mut scenes = vec![];
    let animations = document
        .animations()
        .map(|animation| {
            let channels = animation
                .channels()
                .filter_map(|channel| {
                    println!("{:#?}", channel.target().node().index());
                    println!("{:#?}", channel.target().property());
                    let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
                    let inputs: Vec<f32> = reader.read_inputs().unwrap().collect();
                    let outputs: Vec<Scale> = match reader.read_outputs().unwrap() {
                        gltf::animation::util::ReadOutputs::Scales(outs) => outs
                            .map(|v| Scale {
                                x: v[0],
                                y: v[1],
                                z: v[2],
                            })
                            .collect(),
                        _ => return None,
                    };
                    let input_buffer = Buffer::new_with_init(
                        application,
                        wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::UNIFORM,
                        &inputs,
                        "Input buffer",
                    );
                    let output_buffer = Buffer::new_with_init(
                        application,
                        wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::UNIFORM,
                        &outputs,
                        "Output buffer",
                    );
                    Some(GltfChannel {
                        input_buffer,
                        output_buffer,
                        property: channel.target().property(),
                        target: channel.target().node().index(),
                    })
                })
                .collect();
            GltfAnimation { channels }
        })
        .collect();
    for scene in document.scenes() {
        let mut meshes = vec![];
        for node in scene.nodes() {
            match node.mesh() {
                None => {}
                Some(mesh) => {
                    let mut indices = vec![];
                    let mut vertices = vec![];
                    for primitive in mesh.primitives() {
                        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                        if let Some(positions) = reader.read_positions() {
                            if let Some(normals) = reader.read_normals() {
                                for (position, normal) in positions.zip(normals) {
                                    let color = [255, 255, 0, 255];
                                    vertices.push(MeshVertexAttributes {
                                        position,
                                        normal,
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
                                        indices.push(index as u32);
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
                        vertex_buffer,
                        vertex_count: vertices.len(),
                        index_buffer,
                        index_count: indices.len(),
                    });
                }
            }
        }
        scenes.push(GltfScene { meshes });
    }
    Ok(GltfFile { scenes, animations })
}
