pub mod bloom;
pub mod config;
pub mod outline;
pub mod sky;
pub mod ssao;
pub mod tonemap;

use crate::camera::Camera;
use bloom::BloomPass;
use config::PostProcessConfig;
use outline::OutlinePass;
use sky::SkyPass;
use ssao::SsaoPass;
use tonemap::TonemapPass;

pub struct PostProcessor {
    pub config: PostProcessConfig,
    pub hdr_texture: wgpu::Texture,
    pub hdr_view: wgpu::TextureView,
    hdr_sampler: wgpu::Sampler,
    pub normal_msaa_texture: wgpu::Texture,
    pub normal_msaa_view: wgpu::TextureView,
    pub normal_resolve_texture: wgpu::Texture,
    pub normal_resolve_view: wgpu::TextureView,
    tonemap: TonemapPass,
    outline: OutlinePass,
    sky: SkyPass,
    ssao: Option<SsaoPass>,
    bloom: Option<BloomPass>,
    sample_count: u32,
    _output_format: wgpu::TextureFormat,
}

impl PostProcessor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        output_format: wgpu::TextureFormat,
        camera: &Camera,
        depth_texture_view: &wgpu::TextureView,
        sample_count: u32,
    ) -> Self {
        let (hdr_texture, hdr_view) = Self::create_hdr_target(device, width, height);
        let (normal_msaa_texture, normal_msaa_view) =
            Self::create_normal_msaa_target(device, width, height, sample_count);
        let (normal_resolve_texture, normal_resolve_view) =
            Self::create_normal_resolve_target(device, width, height);

        let hdr_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("HDR sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let config = PostProcessConfig::default();

        let ssao = if config.ssao.is_some() {
            Some(SsaoPass::new(
                device,
                queue,
                width,
                height,
                camera,
                depth_texture_view,
                &normal_resolve_view,
                sample_count,
            ))
        } else {
            None
        };

        let bloom = config
            .bloom
            .as_ref()
            .map(|bloom_config| BloomPass::new(device, width, height, &hdr_view, bloom_config));

        let tonemap = TonemapPass::new(
            device,
            output_format,
            &hdr_view,
            &hdr_sampler,
            ssao.as_ref().map(|s| &s.ssao_blur_view),
            bloom.as_ref().map(|b| b.result_view()),
        );
        let sky = SkyPass::new(
            device,
            wgpu::TextureFormat::Rgba16Float,
            camera,
            sample_count,
        );

        let outline = OutlinePass::new(
            device,
            wgpu::TextureFormat::Rgba16Float,
            &normal_resolve_view,
            depth_texture_view,
        );

        Self {
            config,
            hdr_texture,
            hdr_view,
            hdr_sampler,
            normal_msaa_texture,
            normal_msaa_view,
            normal_resolve_texture,
            normal_resolve_view,
            tonemap,
            outline,
            sky,
            ssao,
            bloom,
            sample_count,
            _output_format: output_format,
        }
    }

    fn create_hdr_target(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("HDR render target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_normal_msaa_target(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        sample_count: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Normal MSAA texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_normal_resolve_target(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Normal resolve texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn rebuild_tonemap(&mut self, device: &wgpu::Device) {
        self.tonemap.rebuild_bind_group(
            device,
            &self.hdr_view,
            &self.hdr_sampler,
            self.ssao.as_ref().map(|s| &s.ssao_blur_view),
            self.bloom.as_ref().map(|b| b.result_view()),
        );
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        depth_texture_view: &wgpu::TextureView,
        camera: &Camera,
    ) {
        let (hdr_texture, hdr_view) = Self::create_hdr_target(device, width, height);
        self.hdr_texture = hdr_texture;
        self.hdr_view = hdr_view;
        let (normal_msaa_texture, normal_msaa_view) =
            Self::create_normal_msaa_target(device, width, height, self.sample_count);
        self.normal_msaa_texture = normal_msaa_texture;
        self.normal_msaa_view = normal_msaa_view;
        let (normal_resolve_texture, normal_resolve_view) =
            Self::create_normal_resolve_target(device, width, height);
        self.normal_resolve_texture = normal_resolve_texture;
        self.normal_resolve_view = normal_resolve_view;

        if let Some(ref mut ssao) = self.ssao {
            ssao.resize(
                device,
                width,
                height,
                depth_texture_view,
                &self.normal_resolve_view,
                camera,
            );
        }

        if let Some(ref mut bloom) = self.bloom {
            let config = self.config.bloom.clone().unwrap_or_default();
            bloom.resize(device, width, height, &self.hdr_view, &config);
        }

        self.outline
            .rebuild_bind_group(device, &self.normal_resolve_view, depth_texture_view);
        self.rebuild_tonemap(device);
    }

    pub fn enable_ssao(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        camera: &Camera,
        depth_texture_view: &wgpu::TextureView,
    ) {
        if self.ssao.is_some() {
            return;
        }
        self.ssao = Some(SsaoPass::new(
            device,
            queue,
            width,
            height,
            camera,
            depth_texture_view,
            &self.normal_resolve_view,
            self.sample_count,
        ));
        self.rebuild_tonemap(device);
    }

    pub fn disable_ssao(&mut self, device: &wgpu::Device) {
        self.ssao = None;
        self.rebuild_tonemap(device);
    }

    pub fn enable_bloom(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.bloom.is_some() {
            return;
        }
        let config = self.config.bloom.clone().unwrap_or_default();
        self.bloom = Some(BloomPass::new(
            device,
            width,
            height,
            &self.hdr_view,
            &config,
        ));
        self.rebuild_tonemap(device);
    }

    pub fn disable_bloom(&mut self, device: &wgpu::Device) {
        self.bloom = None;
        self.rebuild_tonemap(device);
    }

    pub fn render_sky(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        multisampled_framebuffer: &wgpu::TextureView,
        depth_texture: &wgpu::TextureView,
        camera: &Camera,
    ) {
        self.sky.update_params(queue, self.config.sky.mode);
        if self.config.sky.mode == config::SkyMode::Off {
            return;
        }
        self.sky.render(
            encoder,
            multisampled_framebuffer,
            &self.hdr_view,
            depth_texture,
            &self.normal_msaa_view,
            &self.normal_resolve_view,
            camera,
        );
    }

    pub fn render_ssao(&self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        if let Some(ref ssao) = self.ssao {
            if let Some(ref config) = self.config.ssao {
                ssao.update_params(queue, config);
            }
            ssao.render(encoder);
        }
    }

    pub fn render_outline(&self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        if self.config.outline.enabled {
            self.outline.update_params(queue, &self.config.outline);
            self.outline.render(encoder, &self.hdr_view);
        }
    }

    pub fn render_bloom(&self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        if let Some(ref bloom) = self.bloom {
            if let Some(ref config) = self.config.bloom {
                bloom.update_params(queue, config);
            }
            bloom.render(encoder);
        }
    }

    pub fn render_tonemap(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
    ) {
        self.tonemap.update_params(
            queue,
            self.config.tonemapping,
            self.ssao.is_some(),
            self.bloom.is_some(),
        );
        self.tonemap.render(encoder, output_view);
    }
}

impl std::fmt::Debug for PostProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostProcessor")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}
