use std::rc::Rc;
use std::{cell::RefCell, marker::PhantomData};
use uuid::Uuid;

use bytemuck::Pod;
use wgpu::{Device, Queue};

use crate::TextureField;

#[derive(Debug)]
pub struct TextureBufferInner {
    pub label: String,
    pub texture: wgpu::Texture,
    pub sampler: wgpu::Sampler,
    pub handle: Uuid,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub size: wgpu::Extent3d,
}

pub struct TextureBuffer<T: Pod> {
    pub inner: Rc<RefCell<TextureBufferInner>>,
    phantom: PhantomData<T>,
}

impl<T: Pod> TextureBuffer<T> {
    pub fn new(device: &Device, size: wgpu::Extent3d) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture bind group layout"),
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
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Mesh texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Mesh texture sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
            ],
        });
        let label = std::any::type_name::<T>();

        Self {
            inner: Rc::new(RefCell::new(TextureBufferInner {
                label: label.into(),
                texture,
                sampler,
                handle: uuid::Uuid::new_v4(),
                size,
                bind_group,
                bind_group_layout,
            })),
            phantom: PhantomData {},
        }
    }

    pub fn update(&self, _device: &Device, queue: &Queue, size: wgpu::Extent3d, data: &[u8]) {
        let inner = self.inner.borrow_mut();
        log::debug!("Update buffer '{}' with length {}", inner.label, data.len());
        assert_eq!(inner.size, size, "Size change not implemented");
        let wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers,
        } = inner.size;
        assert_eq!(
            (width as usize) * (height as usize) * 4,
            data.len(),
            "RGBA8 data size must be width*height*4"
        );
        assert_eq!(depth_or_array_layers, 1, "Depth != 1 not implemented");
        queue.write_texture(
            inner.texture.as_image_copy(),
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn sample(&self, coordinate: &crate::Expression) -> crate::Expression {
        crate::Expression::TextureField(TextureField {
            handle: self.inner.borrow().handle,
            coordinate: Box::new(coordinate.clone()),
            inner: self.inner.clone(),
        })
    }
}

pub trait TextureDeviceExt {
    fn create_texture_buffer<T>(&self, size: wgpu::Extent3d) -> TextureBuffer<T>
    where
        T: crate::texture_binding::Texture + Pod;
}

impl TextureDeviceExt for wgpu::Device {
    fn create_texture_buffer<T>(&self, size: wgpu::Extent3d) -> TextureBuffer<T>
    where
        T: crate::texture_binding::Texture + Pod,
    {
        TextureBuffer::<T>::new(self, size)
    }
}
