const ACCUMULATION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const WEIGHT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

// Sums color and weight, while the accumulation alpha multiplies up how much of the
// background remains. Both targets share this blend state, since WebGL2 lacks independent blending.
const ACCUMULATION_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

pub fn color_targets() -> [Option<wgpu::ColorTargetState>; 2] {
    [ACCUMULATION_FORMAT, WEIGHT_FORMAT].map(|format| {
        Some(wgpu::ColorTargetState {
            format,
            blend: Some(ACCUMULATION_BLEND),
            write_mask: wgpu::ColorWrites::ALL,
        })
    })
}

struct Target {
    msaa: Option<wgpu::TextureView>,
    resolve: wgpu::TextureView,
}

impl Target {
    fn new(
        device: &wgpu::Device,
        label: &str,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        sample_count: u32,
    ) -> Self {
        let create_view = |sample_count, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        Self {
            msaa: (sample_count > 1)
                .then(|| create_view(sample_count, wgpu::TextureUsages::RENDER_ATTACHMENT)),
            resolve: create_view(
                1,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ),
        }
    }

    fn attachment(&self, clear: Option<wgpu::Color>) -> wgpu::RenderPassColorAttachment<'_> {
        wgpu::RenderPassColorAttachment {
            view: self.msaa.as_ref().unwrap_or(&self.resolve),
            resolve_target: self.msaa.as_ref().map(|_| &self.resolve),
            depth_slice: None,
            ops: wgpu::Operations {
                load: clear.map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                store: wgpu::StoreOp::Store,
            },
        }
    }
}

pub struct TransparencyPass {
    accumulation: Target,
    weight: Target,
    sample_count: u32,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl TransparencyPass {
    pub fn new(
        device: &wgpu::Device,
        hdr_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        sample_count: u32,
    ) -> Self {
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("transparency bind group layout"),
            entries: &[texture_entry(0), texture_entry(1)],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("transparency pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("transparency shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/transparency.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("transparency pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_fullscreen"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: hdr_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let accumulation = Target::new(
            device,
            "transparency accumulation",
            ACCUMULATION_FORMAT,
            width,
            height,
            sample_count,
        );
        let weight = Target::new(
            device,
            "transparency weight",
            WEIGHT_FORMAT,
            width,
            height,
            sample_count,
        );
        let bind_group =
            Self::create_bind_group(device, &bind_group_layout, &accumulation, &weight);

        Self {
            accumulation,
            weight,
            sample_count,
            pipeline,
            bind_group_layout,
            bind_group,
        }
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        accumulation: &Target,
        weight: &Target,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("transparency bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&accumulation.resolve),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&weight.resolve),
                },
            ],
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.accumulation = Target::new(
            device,
            "transparency accumulation",
            ACCUMULATION_FORMAT,
            width,
            height,
            self.sample_count,
        );
        self.weight = Target::new(
            device,
            "transparency weight",
            WEIGHT_FORMAT,
            width,
            height,
            self.sample_count,
        );
        self.bind_group = Self::create_bind_group(
            device,
            &self.bind_group_layout,
            &self.accumulation,
            &self.weight,
        );
    }

    pub fn begin_render_pass<'encoder>(
        &self,
        encoder: &'encoder mut wgpu::CommandEncoder,
        label: &str,
        depth_texture: &wgpu::TextureView,
        clear: bool,
    ) -> wgpu::RenderPass<'encoder> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[
                Some(self.accumulation.attachment(clear.then_some(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }))),
                Some(
                    self.weight
                        .attachment(clear.then_some(wgpu::Color::TRANSPARENT)),
                ),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_texture,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        })
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, hdr_view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("transparency pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: hdr_view,
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

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
