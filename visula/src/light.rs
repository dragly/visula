use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::mem::size_of;

use crate::vec_to_buffer::vec_to_buffer;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LightUniforms {
    pub direction: Vec3,
    pub _pad0: f32,
    pub color: Vec3,
    pub intensity: f32,
    pub light_view_proj: Mat4,
}

unsafe impl Pod for LightUniforms {}
unsafe impl Zeroable for LightUniforms {}

impl AsRef<[f32; size_of::<LightUniforms>() / size_of::<f32>()]> for LightUniforms {
    #[inline]
    fn as_ref(&self) -> &[f32; size_of::<LightUniforms>() / size_of::<f32>()] {
        unsafe {
            &*(self as *const LightUniforms
                as *const [f32; size_of::<LightUniforms>() / size_of::<f32>()])
        }
    }
}

pub const SHADOW_MAP_SIZE: u32 = 4096;

#[derive(Debug)]
pub struct DirectionalLight {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
    pub shadow_texture: wgpu::Texture,
    pub shadow_texture_view: wgpu::TextureView,
    pub shadow_sampler: wgpu::Sampler,
    pub shadow_bind_group_layout: wgpu::BindGroupLayout,
    pub shadow_bind_group: wgpu::BindGroup,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    /// Center of the shadow frustum in world space.
    /// Set this to the camera look-at point each frame.
    pub shadow_center: Vec3,
    /// Half-extent of the orthographic shadow frustum.
    /// Controls how large an area the shadow map covers.
    pub shadow_extent: f32,
    /// Distance from shadow_center to the light source position.
    pub shadow_distance: f32,
}

impl DirectionalLight {
    pub fn new(device: &wgpu::Device) -> Self {
        let direction = Vec3::new(-1.0, -1.0, -1.0).normalize();
        let color = Vec3::ONE;
        let intensity = 1.0;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("light bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(size_of::<LightUniforms>() as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow map"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let shadow_texture_view =
            shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow sampler"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let light_uniforms = [0.0f32; size_of::<LightUniforms>() / size_of::<f32>()];
        let uniform_buffer = vec_to_buffer(
            device,
            light_uniforms.as_ref(),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("light bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });

        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow pass bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(size_of::<LightUniforms>() as u64),
                    },
                    count: None,
                }],
            });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow pass bind group"),
            layout: &shadow_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        DirectionalLight {
            bind_group_layout,
            bind_group,
            uniform_buffer,
            shadow_texture,
            shadow_texture_view,
            shadow_sampler,
            shadow_bind_group_layout,
            shadow_bind_group,
            direction,
            color,
            intensity,
            shadow_center: Vec3::ZERO,
            shadow_extent: 50.0,
            shadow_distance: 200.0,
        }
    }

    pub fn compute_light_view_proj(&self) -> Mat4 {
        let light_pos = self.shadow_center - self.direction * self.shadow_distance;
        let view = Mat4::look_at_rh(light_pos, self.shadow_center, Vec3::Y);
        let e = self.shadow_extent;
        let proj = Mat4::orthographic_rh(-e, e, -e, e, 1.0, self.shadow_distance * 2.0);
        proj * view
    }

    pub fn update(&self, queue: &wgpu::Queue) {
        let light_view_proj = self.compute_light_view_proj();
        let uniforms = LightUniforms {
            direction: self.direction,
            _pad0: 0.0,
            color: self.color,
            intensity: self.intensity,
            light_view_proj,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }
}
