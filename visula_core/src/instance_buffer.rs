use std::rc::Rc;
use std::{cell::RefCell, marker::PhantomData};
use uuid::Uuid;

use bytemuck::Pod;
use wgpu::BufferUsages;
use wgpu::{util::DeviceExt, Device, Queue};

use crate::Instance;

pub struct InstanceBufferInner {
    pub label: String,
    pub buffer: Option<wgpu::Buffer>,
    pub count: usize,
    pub handle: Uuid,
    usage: BufferUsages,
}

pub struct InstanceBuffer<T: Pod> {
    pub inner: Rc<RefCell<InstanceBufferInner>>,
    phantom: PhantomData<T>,
}

impl<T: Instance + Pod> InstanceBuffer<T> {
    pub fn new() -> Self {
        let label = std::any::type_name::<T>();
        let usage = BufferUsages::VERTEX | BufferUsages::COPY_DST;
        Self {
            inner: Rc::new(RefCell::new(InstanceBufferInner {
                label: label.into(),
                buffer: None,
                count: 0,
                usage,
                handle: uuid::Uuid::new_v4(),
            })),
            phantom: PhantomData {},
        }
    }

    pub fn create_buffer(&mut self, device: &wgpu::Device) {
        let mut inner = self.inner.borrow_mut();
        inner.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            size: 0,
            label: Some(inner.label.as_str()),
            usage: inner.usage,
        }));
    }

    pub fn create_buffer_init(&mut self, device: &wgpu::Device, data: &[T]) {
        let mut inner = self.inner.borrow_mut();
        inner.buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(inner.label.as_str()),
                contents: bytemuck::cast_slice(data),
                usage: inner.usage,
            }),
        );
    }

    pub fn update(&mut self, device: &Device, queue: &Queue, data: &[T]) {
        let mut inner = self.inner.borrow_mut();
        log::trace!("Update buffer '{}' with length {}", inner.label, data.len());
        if data.len() == inner.count {
            queue.write_buffer(
                &inner.buffer.as_ref().expect("Buffer not set!"),
                0,
                bytemuck::cast_slice(data),
            );
        } else {
            inner.buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Instance buffer"),
                    contents: bytemuck::cast_slice(data),
                    usage: inner.usage,
                }),
            );
            inner.count = data.len();
        }
    }

    pub fn instance(&self) -> T::Type {
        T::instance(self.inner.clone())
    }
}
