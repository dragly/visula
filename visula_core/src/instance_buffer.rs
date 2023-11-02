use std::rc::Rc;
use std::{cell::RefCell, marker::PhantomData};
use uuid::Uuid;

use bytemuck::Pod;
use wgpu::BufferUsages;
use wgpu::{util::DeviceExt, Device, Queue};

use crate::Instance;

#[derive(Debug)]
pub struct InstanceBufferInner {
    pub label: String,
    pub buffer: wgpu::Buffer,
    pub count: usize,
    pub handle: Uuid,
    usage: BufferUsages,
}

pub struct InstanceBuffer<T: Pod> {
    pub inner: Rc<RefCell<InstanceBufferInner>>,
    phantom: PhantomData<T>,
}

impl<T: Instance + Pod> InstanceBuffer<T> {
    pub fn new(device: &Device) -> Self {
        let usage = BufferUsages::VERTEX | BufferUsages::COPY_DST;
        let label = std::any::type_name::<T>();
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            size: 0,
            label: Some(label),
            usage,
        });

        Self {
            inner: Rc::new(RefCell::new(InstanceBufferInner {
                label: label.into(),
                buffer,
                count: 0,
                usage,
                handle: uuid::Uuid::new_v4(),
            })),
            phantom: PhantomData {},
        }
    }

    pub fn new_with_init(device: &wgpu::Device, data: &[T]) -> Self {
        let label = std::any::type_name::<T>();
        let usage = BufferUsages::VERTEX | BufferUsages::COPY_DST;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage,
        });

        Self {
            inner: Rc::new(RefCell::new(InstanceBufferInner {
                handle: uuid::Uuid::new_v4(),
                label: label.into(),
                buffer,
                count: data.len(),
                usage,
            })),
            phantom: PhantomData {},
        }
    }

    pub fn update(&self, device: &Device, queue: &Queue, data: &[T]) {
        let mut inner = self.inner.borrow_mut();
        log::trace!("Update buffer '{}' with length {}", inner.label, data.len());
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

    pub fn instance(&self) -> T::Type {
        T::instance(self.inner.clone())
    }
}

pub trait InstanceDeviceExt {
    fn create_instance_buffer<T>(&self) -> InstanceBuffer<T>
    where
        T: Instance + Pod;
}

impl InstanceDeviceExt for wgpu::Device {
    fn create_instance_buffer<T>(&self) -> InstanceBuffer<T>
    where
        T: Instance + Pod,
    {
        InstanceBuffer::<T>::new(self)
    }
}
