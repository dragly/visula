use std::cell::Ref;
use std::mem::size_of;

use glam::{Quat, Vec3, Vec4};
use naga::back::wgsl::WriterFlags;
use naga::valid::ValidationFlags;
use wgpu::util::DeviceExt;
use wgpu::{BindGroupLayout, PipelineCompilationOptions};

use crate::pipelines::layouts_by_group;
use crate::post_process::transparency;
use crate::primitives::mesh_primitive::MeshVertexAttributes;
use crate::{RenderData, RenderingDescriptor};
use visula_core::{BindingBuilder, Delegate as _, Expression, InstanceBinding};
use visula_derive::Delegate;

pub struct MeshPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    blending: MeshBlending,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub vertex_count: usize,
    vertex_binding_builder: BindingBuilder,
    fragment_binding_builder: BindingBuilder,
}

#[derive(Delegate)]
pub struct MeshGeometry {
    pub rotation: Expression,
    pub position: Expression,
    pub scale: Expression,
    pub color: Expression,
}

#[derive(Delegate)]
pub struct MeshMaterial {
    pub color: Expression,
}

impl Default for MeshGeometry {
    fn default() -> Self {
        MeshGeometry {
            rotation: Quat::IDENTITY.into(),
            position: Vec3::ZERO.into(),
            scale: Vec3::ONE.into(),
            color: Vec4::ONE.into(),
        }
    }
}

impl Default for MeshMaterial {
    fn default() -> Self {
        MeshMaterial {
            color: Expression::InputColor.lit(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MeshBlending {
    #[default]
    Opaque,
    Transparent,
}

impl MeshPipeline {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        geometry: &MeshGeometry,
        material: &MeshMaterial,
    ) -> Result<Self, visula_core::ShaderError> {
        Self::with_blending(
            rendering_descriptor,
            geometry,
            material,
            MeshBlending::Opaque,
        )
    }

    pub fn with_blending(
        rendering_descriptor: &RenderingDescriptor,
        geometry: &MeshGeometry,
        material: &MeshMaterial,
        blending: MeshBlending,
    ) -> Result<Self, visula_core::ShaderError> {
        let &RenderingDescriptor {
            device,
            camera,
            light,
            format,
            ..
        } = rendering_descriptor;

        let vertex_size = size_of::<MeshVertexAttributes>();

        let shader_with_lighting = format!(
            "{}\n{}",
            visula_core::LIGHTING_WGSL,
            include_str!("../mesh.wgsl"),
        );
        let mut module = naga::front::wgsl::parse_str(&shader_with_lighting)?;
        let info =
            naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
                .validate(&module)
                .map_err(Box::new)?;
        let pre_output_str = naga::back::wgsl::write_string(&module, &info, WriterFlags::all())?;
        log::debug!("Original shader code:\n{pre_output_str}");
        log::debug!("Injecting instance");
        let mut vertex_binding_builder = BindingBuilder::new(&module, "vs_main", 1)?;
        geometry.inject("geometry", &mut module, &mut vertex_binding_builder)?;
        let fragment_entry_point = match blending {
            MeshBlending::Opaque => "fs_main",
            MeshBlending::Transparent => "fs_transparent",
        };
        let mut fragment_binding_builder = BindingBuilder::new(&module, fragment_entry_point, 0)?;
        material.inject_before_use("material", &mut module, &mut fragment_binding_builder)?;

        log::debug!("Validating generated mesh shader\n{module:#?}");
        let info =
            naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
                .validate(&module)
                .map_err(Box::new)?;
        let output_str = naga::back::wgsl::write_string(&module, &info, WriterFlags::all())?;
        log::debug!("Resulting mesh shader code:\n{output_str}");

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mesh shader module"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&output_str)),
        });

        let delegate_bind_group_layouts: Vec<(u32, BindGroupLayout)> = vertex_binding_builder
            .bind_group_layouts()
            .into_iter()
            .chain(fragment_binding_builder.bind_group_layouts())
            .collect();

        let bind_group_layouts = layouts_by_group(
            &[&camera.bind_group_layout, &light.bind_group_layout],
            &delegate_bind_group_layouts,
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh pipeline layout"),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3, // position
                1 => Float32x3, // normal
                2 => Float32x2, // uv
                3 => Unorm8x4,  // color
            ],
        };

        let mut layouts = vertex_binding_builder
            .instances
            .values()
            .map(|binding| binding.layout.build())
            .collect();

        let buffers = {
            let mut buffers = vec![vertex_buffer_layout];
            buffers.append(&mut layouts);
            buffers
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &buffers,
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some(fragment_entry_point),
                targets: &match blending {
                    MeshBlending::Opaque => [
                        Some(wgpu::ColorTargetState {
                            format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba16Float,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                    MeshBlending::Transparent => transparency::color_targets(),
                },
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(blending == MeshBlending::Opaque),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: rendering_descriptor.sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh vertex buffer"),
            contents: bytemuck::cast_slice(&Vec::<MeshVertexAttributes>::new()),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh index buffer"),
            contents: bytemuck::cast_slice(&Vec::<u32>::new()),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(MeshPipeline {
            render_pipeline,
            blending,
            vertex_buffer,
            index_buffer,
            vertex_count: 0,
            vertex_binding_builder,
            fragment_binding_builder,
        })
    }

    pub fn set_mesh_data(
        &mut self,
        device: &wgpu::Device,
        vertices: &[MeshVertexAttributes],
        indices: &[u32],
    ) {
        self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh vertex buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh index buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.vertex_count = indices.len();
    }

    pub fn render(&self, data: &mut RenderData) {
        log::debug!("Rendering meshes");
        if self.vertex_count == 0 {
            return;
        }

        let mut count = None;
        for binding in self.vertex_binding_builder.instances.values() {
            let other = binding.inner.borrow().count;
            if other == 0 {
                return;
            }
            count = match count {
                None => Some(other),
                Some(old) => {
                    if other != old {
                        None
                    } else {
                        Some(old)
                    }
                }
            }
        }
        log::debug!("Mesh count {count:#?}");

        let instances: Vec<(&InstanceBinding, Ref<wgpu::Buffer>)> = self
            .vertex_binding_builder
            .instances
            .values()
            .map(|v| (v, Ref::map(v.inner.borrow(), |v| &v.buffer)))
            .collect();

        let bind_groups: Vec<(u32, wgpu::BindGroup)> = self
            .vertex_binding_builder
            .bind_groups()
            .into_iter()
            .chain(self.fragment_binding_builder.bind_groups())
            .collect();

        let (camera, light) = (data.camera, data.light);
        let render_pass = match self.blending {
            MeshBlending::Opaque => &mut data.pass,
            MeshBlending::Transparent => data.transparent_pass(),
        };

        render_pass.push_debug_group("meshes");
        render_pass.set_bind_group(0, &camera.bind_group, &[]);
        render_pass.set_bind_group(1, &light.bind_group, &[]);

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        let mut instance_count = usize::from(count.is_none());
        for (binding, buffer) in instances.iter() {
            let slot = binding.slot;
            log::debug!("Setting vertex buffer {slot}");
            render_pass.set_vertex_buffer(slot, buffer.slice(..));
            instance_count = instance_count.max(binding.inner.borrow().count);
        }

        for (group, bind_group) in &bind_groups {
            log::debug!("Setting bind group {group}");
            render_pass.set_bind_group(*group, bind_group, &[]);
        }

        log::debug!("Drawing {instance_count} instances");
        render_pass.draw_indexed(0..self.vertex_count as u32, 0, 0..instance_count as u32);
        render_pass.pop_debug_group();
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::{Pod, Zeroable};
    use visula_core::{TextureBuffer, UniformBuffer};
    use visula_derive::Uniform;

    use super::*;
    use crate::pipelines::test_util::TestScene;

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable, Uniform)]
    struct Offset {
        position: [f32; 3],
        _padding: f32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable, Uniform)]
    struct Tint {
        color: [f32; 4],
    }

    #[test]
    fn textured_mesh_with_uniforms_binds_every_group_to_its_layout() {
        let scene = TestScene::new();
        let device = &scene.device;
        let offset = UniformBuffer::<Offset>::new(device).uniform();
        let tint = UniformBuffer::<Tint>::new(device).uniform();
        let texture = TextureBuffer::<f32>::new(
            device,
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let mut mesh = MeshPipeline::new(
            &scene.rendering_descriptor(),
            &MeshGeometry {
                position: offset.position,
                ..Default::default()
            },
            &MeshMaterial {
                color: (tint.color * texture.sample(&Expression::UV)).lit(),
            },
        )
        .unwrap();
        let vertex = MeshVertexAttributes {
            position: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0; 2],
            color: [255; 4],
        };
        mesh.set_mesh_data(device, &[vertex; 3], &[0, 1, 2]);

        scene.render(|data| mesh.render(data));
    }
}
