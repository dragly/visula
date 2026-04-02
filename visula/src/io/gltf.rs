use std::io::Read;
use std::io::Seek;
use wgpu::util::DeviceExt;

use crate::error::Error;
use crate::primitives::mesh_primitive::MeshVertexAttributes;
use crate::Application;
use visula_core::TextureBuffer;

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
    pub texture: Option<TextureBuffer<u8>>,
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

fn load_texture(
    document: &gltf::Document,
    buffers: &[Vec<u8>],
    material_index: Option<usize>,
    application: &Application,
) -> Option<TextureBuffer<u8>> {
    let material = document.materials().nth(material_index?)?;
    let texture_info = material.pbr_metallic_roughness().base_color_texture()?;
    let image = texture_info.texture().source();
    let image_data = match image.source() {
        gltf::image::Source::View { view, .. } => {
            let parent = &buffers[view.buffer().index()];
            let begin = view.offset();
            let end = begin + view.length();
            &parent[begin..end]
        }
        gltf::image::Source::Uri { .. } => return None,
    };
    let decoded = image::load_from_memory(image_data).ok()?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = TextureBuffer::<u8>::new(&application.device, size);
    texture.update(&application.device, &application.queue, size, &rgba);
    Some(texture)
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
        document: &gltf::Document,
        buffers: &[Vec<u8>],
        application: &Application,
        meshes: &mut Vec<GltfMesh>,
    ) {
        if let Some(mesh) = node.mesh() {
            let mut indices = vec![];
            let mut vertices = vec![];
            let mut material_index = None;
            for primitive in mesh.primitives() {
                material_index = primitive.material().index();
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                if let Some(positions) = reader.read_positions() {
                    if let Some(normals) = reader.read_normals() {
                        let colors: Vec<[u8; 4]> = reader
                            .read_colors(0)
                            .map(|c| c.into_rgba_u8().collect())
                            .unwrap_or_default();
                        let uvs: Vec<[f32; 2]> = reader
                            .read_tex_coords(0)
                            .map(|t| t.into_f32().collect())
                            .unwrap_or_default();
                        for (i, (position, normal)) in positions.zip(normals).enumerate() {
                            let color = colors.get(i).copied().unwrap_or([255, 255, 255, 255]);
                            let uv = uvs.get(i).copied().unwrap_or([0.0, 0.0]);
                            vertices.push(MeshVertexAttributes {
                                position,
                                normal,
                                uv,
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
            let texture = load_texture(document, buffers, material_index, application);
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
                texture,
            });
        }
        for child in node.children() {
            collect_meshes(child, document, buffers, application, meshes);
        }
    }

    let mut scenes = vec![];
    for scene in document.scenes() {
        let mut meshes = vec![];
        for node in scene.nodes() {
            collect_meshes(node, &document, &buffers, application, &mut meshes);
        }
        scenes.push(GltfScene { meshes });
    }
    Ok(GltfFile { scenes })
}
