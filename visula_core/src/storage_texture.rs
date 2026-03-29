use wgpu::Device;

pub struct StorageTexture {
    pub texture: wgpu::Texture,
    pub compute_bind_group: wgpu::BindGroup,
    pub compute_bind_group_layout: wgpu::BindGroupLayout,
    pub sample_bind_group: wgpu::BindGroup,
    pub sample_bind_group_layout: wgpu::BindGroupLayout,
    pub width: u32,
    pub height: u32,
}

impl StorageTexture {
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("StorageTexture compute layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });

        let sample_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("StorageTexture sample layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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

        let (texture, compute_bind_group, sample_bind_group) =
            Self::create_texture_and_bind_groups(
                device,
                width,
                height,
                &compute_bind_group_layout,
                &sample_bind_group_layout,
            );

        Self {
            texture,
            compute_bind_group,
            compute_bind_group_layout,
            sample_bind_group,
            sample_bind_group_layout,
            width,
            height,
        }
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }
        self.width = width;
        self.height = height;
        let (texture, compute_bg, sample_bg) = Self::create_texture_and_bind_groups(
            device,
            width,
            height,
            &self.compute_bind_group_layout,
            &self.sample_bind_group_layout,
        );
        self.texture = texture;
        self.compute_bind_group = compute_bg;
        self.sample_bind_group = sample_bg;
    }

    fn create_texture_and_bind_groups(
        device: &Device,
        width: u32,
        height: u32,
        compute_layout: &wgpu::BindGroupLayout,
        sample_layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::Texture, wgpu::BindGroup, wgpu::BindGroup) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("StorageTexture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("StorageTexture compute bind group"),
            layout: compute_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("StorageTexture sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let sample_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("StorageTexture sample bind group"),
            layout: sample_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
            ],
        });

        (texture, compute_bind_group, sample_bind_group)
    }
}
