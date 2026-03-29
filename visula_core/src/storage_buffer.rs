use std::marker::PhantomData;

use bytemuck::Pod;
use uuid::Uuid;
use wgpu::BufferUsages;
use wgpu::{util::DeviceExt, Device, Queue};

pub struct StorageBufferInner {
    pub label: String,
    pub buffer: wgpu::Buffer,
    pub count: usize,
    pub handle: Uuid,
    usage: BufferUsages,
}

pub struct StorageBuffer<T: Pod> {
    pub inner: StorageBufferInner,
    phantom: PhantomData<T>,
}

impl<T: Pod> StorageBuffer<T> {
    pub fn new(device: &Device, label: &str) -> Self {
        let usage = BufferUsages::STORAGE | BufferUsages::COPY_DST;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            size: std::mem::size_of::<T>() as u64,
            label: Some(label),
            usage,
        });

        Self {
            inner: StorageBufferInner {
                label: label.into(),
                buffer,
                count: 0,
                usage,
                handle: Uuid::new_v4(),
            },
            phantom: PhantomData,
        }
    }

    pub fn new_with_init(device: &Device, label: &str, data: &[T]) -> Self {
        let usage = BufferUsages::STORAGE | BufferUsages::COPY_DST;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage,
        });

        Self {
            inner: StorageBufferInner {
                label: label.into(),
                buffer,
                count: data.len(),
                usage,
                handle: Uuid::new_v4(),
            },
            phantom: PhantomData,
        }
    }

    pub fn update(&mut self, device: &Device, queue: &Queue, data: &[T]) {
        if data.len() == self.inner.count && self.inner.count > 0 {
            queue.write_buffer(&self.inner.buffer, 0, bytemuck::cast_slice(data));
        } else {
            self.inner.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&self.inner.label),
                contents: bytemuck::cast_slice(data),
                usage: self.inner.usage,
            });
            self.inner.count = data.len();
            self.inner.handle = Uuid::new_v4();
        }
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.inner.buffer
    }

    pub fn handle(&self) -> Uuid {
        self.inner.handle
    }
}
