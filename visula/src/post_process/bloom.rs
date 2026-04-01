use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::config::BloomConfig;

const MAX_MIP_LEVELS: usize = 6;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DownsampleParams {
    texel_size: [f32; 2],
    threshold: f32,
    is_first_pass: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct UpsampleParams {
    texel_size: [f32; 2],
    intensity: f32,
    _pad: f32,
}

struct MipLevel {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

pub struct BloomPass {
    downsample_pipeline: wgpu::RenderPipeline,
    upsample_pipeline: wgpu::RenderPipeline,
    downsample_bind_group_layout: wgpu::BindGroupLayout,
    upsample_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    mip_levels: Vec<MipLevel>,
    downsample_bind_groups: Vec<wgpu::BindGroup>,
    downsample_params_buffers: Vec<wgpu::Buffer>,
    upsample_bind_groups: Vec<wgpu::BindGroup>,
    upsample_params_buffers: Vec<wgpu::Buffer>,
    mip_count: usize,
    source_width: u32,
    source_height: u32,
}

impl BloomPass {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        hdr_view: &wgpu::TextureView,
        config: &BloomConfig,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Bloom sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let downsample_bind_group_layout =
            Self::create_bind_group_layout(device, "bloom downsample");
        let upsample_bind_group_layout = Self::create_bind_group_layout(device, "bloom upsample");

        let downsample_pipeline = Self::create_pipeline(
            device,
            &downsample_bind_group_layout,
            "bloom_downsample",
            include_str!("../shaders/bloom_downsample.wgsl"),
            false,
        );
        let upsample_pipeline = Self::create_pipeline(
            device,
            &upsample_bind_group_layout,
            "bloom_upsample",
            include_str!("../shaders/bloom_upsample.wgsl"),
            true,
        );

        let mip_count = config.mip_levels.min(MAX_MIP_LEVELS as u32) as usize;
        let mip_levels = Self::create_mip_chain(device, width, height, mip_count);

        let downsample_params_buffers =
            Self::create_downsample_params_buffers(device, width, height, &mip_levels, config);

        let downsample_bind_groups = Self::create_downsample_bind_groups(
            device,
            &downsample_bind_group_layout,
            &sampler,
            hdr_view,
            &mip_levels,
            &downsample_params_buffers,
        );

        let upsample_params_buffers =
            Self::create_upsample_params_buffers(device, &mip_levels, config);

        let upsample_bind_groups = Self::create_upsample_bind_groups(
            device,
            &upsample_bind_group_layout,
            &sampler,
            &mip_levels,
            &upsample_params_buffers,
        );

        Self {
            downsample_pipeline,
            upsample_pipeline,
            downsample_bind_group_layout,
            upsample_bind_group_layout,
            sampler,
            mip_levels,
            downsample_bind_groups,
            downsample_params_buffers,
            upsample_bind_groups,
            upsample_params_buffers,
            mip_count,
            source_width: width,
            source_height: height,
        }
    }

    fn create_bind_group_layout(device: &wgpu::Device, label: &str) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{label} bind group layout")),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        label: &str,
        shader_source: &str,
        additive_blend: bool,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{label} shader")),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{label} pipeline layout")),
            bind_group_layouts: &[Some(bind_group_layout)],
            immediate_size: 0,
        });

        let blend = if additive_blend {
            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::REPLACE,
            })
        } else {
            None
        };

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{label} pipeline")),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    fn create_mip_chain(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        mip_count: usize,
    ) -> Vec<MipLevel> {
        let mut levels = Vec::new();
        let mut w = width / 2;
        let mut h = height / 2;
        for i in 0..mip_count {
            w = w.max(1);
            h = h.max(1);
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("Bloom mip {i}")),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            levels.push(MipLevel {
                _texture: texture,
                view,
                width: w,
                height: h,
            });
            w /= 2;
            h /= 2;
        }
        levels
    }

    fn create_downsample_params_buffers(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        mip_levels: &[MipLevel],
        config: &BloomConfig,
    ) -> Vec<wgpu::Buffer> {
        let mut buffers = Vec::new();
        for (i, _mip) in mip_levels.iter().enumerate() {
            let (src_w, src_h) = if i == 0 {
                (width, height)
            } else {
                (mip_levels[i - 1].width, mip_levels[i - 1].height)
            };
            let params = DownsampleParams {
                texel_size: [1.0 / src_w as f32, 1.0 / src_h as f32],
                threshold: config.threshold,
                is_first_pass: if i == 0 { 1 } else { 0 },
            };
            buffers.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Bloom downsample params {i}")),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }
        buffers
    }

    fn create_downsample_bind_groups(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        hdr_view: &wgpu::TextureView,
        mip_levels: &[MipLevel],
        params_buffers: &[wgpu::Buffer],
    ) -> Vec<wgpu::BindGroup> {
        let mut groups = Vec::new();
        for (i, _mip) in mip_levels.iter().enumerate() {
            let src_view = if i == 0 {
                hdr_view
            } else {
                &mip_levels[i - 1].view
            };

            groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("Bloom downsample bind group {i}")),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(src_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffers[i].as_entire_binding(),
                    },
                ],
            }));
        }
        groups
    }

    fn create_upsample_params_buffers(
        device: &wgpu::Device,
        mip_levels: &[MipLevel],
        config: &BloomConfig,
    ) -> Vec<wgpu::Buffer> {
        let mut buffers = Vec::new();
        if mip_levels.len() < 2 {
            return buffers;
        }
        for i in (0..mip_levels.len() - 1).rev() {
            let src = &mip_levels[i + 1];
            let params = UpsampleParams {
                texel_size: [1.0 / src.width as f32, 1.0 / src.height as f32],
                intensity: config.intensity,
                _pad: 0.0,
            };
            buffers.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Bloom upsample params {i}")),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }
        buffers
    }

    fn create_upsample_bind_groups(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        mip_levels: &[MipLevel],
        params_buffers: &[wgpu::Buffer],
    ) -> Vec<wgpu::BindGroup> {
        let mut groups = Vec::new();
        if mip_levels.len() < 2 {
            return groups;
        }
        for (pass_idx, buffer) in params_buffers.iter().enumerate() {
            let src_idx = mip_levels.len() - 1 - pass_idx;
            let src = &mip_levels[src_idx];

            groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("Bloom upsample bind group {pass_idx}")),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffer.as_entire_binding(),
                    },
                ],
            }));
        }
        groups
    }

    pub fn result_view(&self) -> &wgpu::TextureView {
        &self.mip_levels[0].view
    }

    pub fn update_params(&self, queue: &wgpu::Queue, config: &BloomConfig) {
        for (i, buffer) in self.downsample_params_buffers.iter().enumerate() {
            let (src_w, src_h) = if i == 0 {
                (self.source_width, self.source_height)
            } else {
                (self.mip_levels[i - 1].width, self.mip_levels[i - 1].height)
            };
            let params = DownsampleParams {
                texel_size: [1.0 / src_w as f32, 1.0 / src_h as f32],
                threshold: config.threshold,
                is_first_pass: if i == 0 { 1 } else { 0 },
            };
            queue.write_buffer(buffer, 0, bytemuck::bytes_of(&params));
        }

        for (i, buffer) in self.upsample_params_buffers.iter().enumerate() {
            let src_idx = self.mip_count - 1 - i;
            if src_idx < self.mip_levels.len() {
                let src = &self.mip_levels[src_idx];
                let params = UpsampleParams {
                    texel_size: [1.0 / src.width as f32, 1.0 / src.height as f32],
                    intensity: config.intensity,
                    _pad: 0.0,
                };
                queue.write_buffer(buffer, 0, bytemuck::bytes_of(&params));
            }
        }
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        hdr_view: &wgpu::TextureView,
        config: &BloomConfig,
    ) {
        self.source_width = width;
        self.source_height = height;
        self.mip_levels = Self::create_mip_chain(device, width, height, self.mip_count);
        self.downsample_params_buffers =
            Self::create_downsample_params_buffers(device, width, height, &self.mip_levels, config);
        self.downsample_bind_groups = Self::create_downsample_bind_groups(
            device,
            &self.downsample_bind_group_layout,
            &self.sampler,
            hdr_view,
            &self.mip_levels,
            &self.downsample_params_buffers,
        );
        self.upsample_params_buffers =
            Self::create_upsample_params_buffers(device, &self.mip_levels, config);
        self.upsample_bind_groups = Self::create_upsample_bind_groups(
            device,
            &self.upsample_bind_group_layout,
            &self.sampler,
            &self.mip_levels,
            &self.upsample_params_buffers,
        );
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder) {
        for (i, mip) in self.mip_levels.iter().enumerate() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("Bloom downsample {i}")),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &mip.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.downsample_pipeline);
            pass.set_bind_group(0, &self.downsample_bind_groups[i], &[]);
            pass.draw(0..3, 0..1);
        }

        for (pass_idx, bind_group) in self.upsample_bind_groups.iter().enumerate() {
            let target_idx = self.mip_count - 2 - pass_idx;
            let target = &self.mip_levels[target_idx];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("Bloom upsample {pass_idx}")),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.upsample_pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
