use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::config::Tonemapping;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TonemapParams {
    mode: u32,
    ssao_enabled: u32,
    bloom_enabled: u32,
    _pad0: u32,
}

pub struct TonemapPass {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    params_buffer: wgpu::Buffer,
    dummy_r8_view: wgpu::TextureView,
    dummy_rgba_view: wgpu::TextureView,
}

#[allow(clippy::too_many_arguments)]
impl TonemapPass {
    pub fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        hdr_view: &wgpu::TextureView,
        hdr_sampler: &wgpu::Sampler,
        ssao_view: Option<&wgpu::TextureView>,
        bloom_view: Option<&wgpu::TextureView>,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tonemap bind group layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let params = TonemapParams {
            mode: 2,
            ssao_enabled: if ssao_view.is_some() { 1 } else { 0 },
            bloom_enabled: if bloom_view.is_some() { 1 } else { 0 },
            _pad0: 0,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tonemap params buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let dummy_r8_view = Self::create_dummy_texture(device, wgpu::TextureFormat::R8Unorm);
        let dummy_rgba_view = Self::create_dummy_texture(device, wgpu::TextureFormat::Rgba16Float);

        let actual_ssao = ssao_view.unwrap_or(&dummy_r8_view);
        let actual_bloom = bloom_view.unwrap_or(&dummy_rgba_view);

        let bind_group = Self::create_bind_group(
            device,
            &bind_group_layout,
            hdr_view,
            hdr_sampler,
            &params_buffer,
            actual_ssao,
            actual_bloom,
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tonemap pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tonemap shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/tonemap.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tonemap pipeline"),
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
                    format: output_format,
                    blend: None,
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

        Self {
            pipeline,
            bind_group_layout,
            bind_group,
            params_buffer,
            dummy_r8_view,
            dummy_rgba_view,
        }
    }

    fn create_dummy_texture(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dummy texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        hdr_view: &wgpu::TextureView,
        hdr_sampler: &wgpu::Sampler,
        params_buffer: &wgpu::Buffer,
        ssao_view: &wgpu::TextureView,
        bloom_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tonemap bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(hdr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(ssao_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(bloom_view),
                },
            ],
        })
    }

    pub fn rebuild_bind_group(
        &mut self,
        device: &wgpu::Device,
        hdr_view: &wgpu::TextureView,
        hdr_sampler: &wgpu::Sampler,
        ssao_view: Option<&wgpu::TextureView>,
        bloom_view: Option<&wgpu::TextureView>,
    ) {
        let actual_ssao = ssao_view.unwrap_or(&self.dummy_r8_view);
        let actual_bloom = bloom_view.unwrap_or(&self.dummy_rgba_view);
        self.bind_group = Self::create_bind_group(
            device,
            &self.bind_group_layout,
            hdr_view,
            hdr_sampler,
            &self.params_buffer,
            actual_ssao,
            actual_bloom,
        );
    }

    pub fn update_params(
        &self,
        queue: &wgpu::Queue,
        tonemapping: Tonemapping,
        ssao_enabled: bool,
        bloom_enabled: bool,
    ) {
        let mode = match tonemapping {
            Tonemapping::None => 0u32,
            Tonemapping::Reinhard => 1,
            Tonemapping::AcesFilmic => 2,
        };
        let params = TonemapParams {
            mode,
            ssao_enabled: if ssao_enabled { 1 } else { 0 },
            bloom_enabled: if bloom_enabled { 1 } else { 0 },
            _pad0: 0,
        };
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("tonemap pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
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

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
