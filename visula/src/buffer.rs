use std::rc::Rc;
use std::{cell::RefCell, marker::PhantomData};
use uuid::Uuid;

use bytemuck::Pod;
use wgpu::{util::DeviceExt, BufferUsages};
use wgpu::{Device, Queue};

use crate::{Instance, Uniform};

pub struct BufferInner {
    pub label: String,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: Rc<wgpu::BindGroupLayout>,
    pub count: usize,
    pub handle: Uuid,
    usage: BufferUsages,
}

pub struct Buffer<T: Pod> {
    pub inner: Rc<RefCell<BufferInner>>,
    phantom: PhantomData<T>,
}

impl<T: Pod> Buffer<T> {
    pub fn new(device: &Device) -> Self {
        let usage = BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::COPY_DST;
        let label = std::any::type_name::<T>();
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: true, // TODO not sure we need this?
            size: 16,
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

        Buffer {
            inner: Rc::new(RefCell::new(BufferInner {
                label: label.into(),
                buffer,
                count: 0,
                usage,
                handle: uuid::Uuid::new_v4(),
                bind_group,
                bind_group_layout: Rc::new(bind_group_layout),
            })),
            phantom: PhantomData {},
        }
    }

    pub fn new_with_init(device: &wgpu::Device, data: &[T]) -> Self {
        let label = std::any::type_name::<T>();
        let usage = BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::COPY_DST;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
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

        Buffer {
            inner: Rc::new(RefCell::new(BufferInner {
                handle: uuid::Uuid::new_v4(),
                label: label.into(),
                buffer,
                count: data.len(),
                usage,
                bind_group,
                bind_group_layout: Rc::new(bind_group_layout),
            })),
            phantom: PhantomData {},
        }
    }

    pub fn update(&mut self, device: &Device, queue: &Queue, data: &[T]) {
        let mut inner = self.inner.borrow_mut();
        log::debug!("Update buffer '{}' with length {}", inner.label, data.len());
        if data.len() == inner.count {
            queue.write_buffer(&inner.buffer, 0, bytemuck::cast_slice(data));
        } else {
            inner.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance buffer"),
                contents: bytemuck::cast_slice(data),
                usage: inner.usage,
            });
            inner.count = data.len();
        }
    }
}

impl<T: Instance + Pod> Buffer<T> {
    // TODO move T to Buffer<T>
    pub fn instance(&self) -> T::Type {
        T::instance(self.inner.clone())
    }
}

impl<T: Uniform + Pod> Buffer<T> {
    // TODO move T to Buffer<T>
    pub fn uniform(&self) -> T::Type {
        T::uniform(self.inner.clone())
    }
}
