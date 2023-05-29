use std::rc::Rc;
use std::{cell::RefCell, marker::PhantomData};
use uuid::Uuid;

use bytemuck::Pod;
use wgpu::{util::DeviceExt, BufferUsages};
use wgpu::{Device, Queue};

use crate::Uniform;

pub struct UniformBufferInner {
    pub label: String,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: Rc<wgpu::BindGroupLayout>,
    pub handle: Uuid,
}

pub struct UniformBuffer<T: Pod> {
    pub inner: Rc<RefCell<UniformBufferInner>>,
    phantom: PhantomData<T>,
}

impl<T: Pod> UniformBuffer<T> {
    pub fn new(device: &Device) -> Self {
        let usage = BufferUsages::UNIFORM | BufferUsages::COPY_DST;
        let label = std::any::type_name::<T>();
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            size: std::mem::size_of::<T>() as u64,
            label: Some(label),
            usage,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            inner: Rc::new(RefCell::new(UniformBufferInner {
                label: label.into(),
                buffer,
                handle: uuid::Uuid::new_v4(),
                bind_group,
                bind_group_layout: Rc::new(bind_group_layout),
            })),
            phantom: PhantomData {},
        }
    }

    pub fn new_with_init(device: &wgpu::Device, data: &T) -> Self {
        let label = std::any::type_name::<T>();
        let usage = BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::COPY_DST;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&[*data]),
            usage,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<T>() as u64),
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            inner: Rc::new(RefCell::new(UniformBufferInner {
                handle: uuid::Uuid::new_v4(),
                label: label.into(),
                buffer,
                bind_group,
                bind_group_layout: Rc::new(bind_group_layout),
            })),
            phantom: PhantomData {},
        }
    }

    pub fn update(&mut self, queue: &Queue, data: &T) {
        let inner = self.inner.borrow_mut();
        log::debug!("Update uniform buffer '{}'", inner.label);
        queue.write_buffer(&inner.buffer, 0, bytemuck::cast_slice(&[*data]));
    }
}

impl<T: Uniform + Pod> UniformBuffer<T> {
    // TODO move T to Buffer<T>
    pub fn uniform(&self) -> T::Type {
        T::uniform(self.inner.clone())
    }
}
