use crate::pipelines::quad::{QuadPipeline, QuadPipelineDescriptor};
use crate::rendering_descriptor::RenderingDescriptor;
use crate::simulation::RenderData;
use crate::text::{GlyphAtlas, DISTANCE_RANGE};
use crate::Renderable;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec4};
use std::mem::size_of;
use visula_core::Expression;
use visula_derive::Delegate;

const SURFACE_DEPTH_BIAS: wgpu::DepthBiasState = wgpu::DepthBiasState {
    constant: -8,
    slope_scale: -1.0,
    clamp: 0.0,
};

/// A corner of a glyph quad: its position in the plane of the text
/// and the matching texel in the [`GlyphAtlas`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct GlyphVertex {
    pub position: [f32; 2],
    pub texel: [f32; 2],
}

/// Draws text laid out by a [`GlyphAtlas`] from its multi-channel signed distance fields,
/// so the edges stay sharp and smooth at any zoom.
/// The text is pulled slightly toward the camera so it stays in front of the surface it lies on.
pub struct Glyphs {
    pipeline: QuadPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    texture: Option<wgpu::Texture>,
    revision: Option<u64>,
}

#[derive(Delegate)]
pub struct GlyphDelegate {
    pub color: Expression,
    pub transform: Expression,
}

impl Default for GlyphDelegate {
    fn default() -> Self {
        GlyphDelegate {
            color: Vec4::new(1.0, 1.0, 1.0, 1.0).into(),
            transform: Mat4::IDENTITY.into(),
        }
    }
}

impl Glyphs {
    pub fn new(
        rendering_descriptor: &RenderingDescriptor,
        delegate: &GlyphDelegate,
    ) -> Result<Self, visula_core::ShaderError> {
        let device = rendering_descriptor.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyph atlas bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glyph atlas sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shader_source = format!(
            "const DISTANCE_RANGE: f32 = {DISTANCE_RANGE:?};\n{}",
            include_str!("../shaders/glyph.wgsl")
        );
        let pipeline = QuadPipeline::new(
            rendering_descriptor,
            &QuadPipelineDescriptor {
                label: "glyphs",
                shader_source: &shader_source,
                shader_variable_name: "glyph",
                fragment_shader_variable_name: None,
                shadow_shader_source: None,
                vertex_data: &[],
                vertex_stride: size_of::<GlyphVertex>(),
                vertex_format: wgpu::VertexFormat::Float32x4,
                index_data: &[],
                index_format: wgpu::IndexFormat::Uint32,
                bind_group_layouts: &[&layout],
                depth_write_enabled: false,
                depth_bias: SURFACE_DEPTH_BIAS,
            },
            delegate,
            None,
        )?;
        Ok(Glyphs {
            pipeline,
            layout,
            sampler,
            texture: None,
            revision: None,
        })
    }

    pub fn update_geometry(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[GlyphVertex],
        indices: &[u32],
    ) {
        self.pipeline.update_geometry(
            device,
            queue,
            bytemuck::cast_slice(vertices),
            bytemuck::cast_slice(indices),
        );
    }

    /// Uploads the atlas if it changed since the last upload.
    pub fn update_atlas(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, atlas: &GlyphAtlas) {
        if self.revision == Some(atlas.revision()) {
            return;
        }
        let size = wgpu::Extent3d {
            width: atlas.width(),
            height: atlas.height(),
            depth_or_array_layers: 1,
        };
        let texture = match &self.texture {
            Some(texture) if texture.size() == size => texture.clone(),
            _ => {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("glyph atlas"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                self.pipeline.set_bind_group(
                    0,
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("glyph atlas bind group"),
                        layout: &self.layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&self.sampler),
                            },
                        ],
                    }),
                );
                self.texture = Some(texture.clone());
                texture
            }
        };
        queue.write_texture(
            texture.as_image_copy(),
            atlas.pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.width),
                rows_per_image: Some(size.height),
            },
            size,
        );
        self.revision = Some(atlas.revision());
    }
}

impl Renderable for Glyphs {
    fn render(&self, render_data: &mut RenderData) {
        self.pipeline.render(render_data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use crate::light::DirectionalLight;

    #[test]
    fn uploads_text_and_a_growing_atlas() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let camera = Camera::new(&device);
        let light = DirectionalLight::new(&device);
        let rendering_descriptor = RenderingDescriptor {
            device: &device,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            camera: &camera,
            light: &light,
            sample_count: 1,
        };
        let mut glyphs = Glyphs::new(&rendering_descriptor, &GlyphDelegate::default()).unwrap();
        let font = egui::FontDefinitions::default().font_data["Ubuntu-Light"]
            .font
            .to_vec();
        let mut atlas = GlyphAtlas::new(&font).unwrap();
        for text in ["Hello", &(' '..='~').chain('¡'..='ÿ').collect::<String>()] {
            let wrapped = atlas.lay_out_wrapped(text, 1.0, 10.0).unwrap();
            glyphs.update_atlas(&device, &queue, &atlas);
            glyphs.update_geometry(
                &device,
                &queue,
                &wrapped.geometry.vertices,
                &wrapped.geometry.indices,
            );
        }
        queue.submit([]);
    }
}
