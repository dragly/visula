use std::marker::PhantomData;
use std::rc::Rc;

use bytemuck::Pod;
use wgpu::{util::DeviceExt, BufferUsages};

use crate::{Application, Instance, InstanceBinding, Uniform};

pub struct Buffer<T: Pod> {
    pub label: String,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: Rc<wgpu::BindGroupLayout>,
    pub count: usize,
    pub handle: u64,
    usage: BufferUsages,
    phantom: PhantomData<T>,
}

impl<T: Pod> Buffer<T> {
    pub fn new(application: &mut crate::Application, usage: BufferUsages, label: &str) -> Self {
        let buffer = application.device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: true, // TODO not sure we need this?
            size: 16,
            label: Some(label),
            usage,
        });

        let handle = application.create_buffer_handle();

        let bind_group_layout =
            application
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let bind_group = application
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

        Buffer {
            label: label.into(),
            buffer,
            count: 0,
            usage,
            handle,
            phantom: PhantomData {},
            bind_group,
            bind_group_layout: Rc::new(bind_group_layout),
        }
    }

    pub fn new_with_init(
        application: &mut crate::Application,
        usage: BufferUsages,
        data: &[T],
        label: &str,
    ) -> Self {
        let buffer = application
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage,
            });

        let handle = application.create_buffer_handle();

        let bind_group_layout = application.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
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
            },
        );

        let bind_group = application
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

        Buffer {
            label: label.into(),
            buffer,
            count: data.len(),
            usage,
            handle,
            phantom: PhantomData {},
            bind_group,
            bind_group_layout: Rc::new(bind_group_layout),
        }
    }

    pub fn update(&mut self, application: &Application, data: &[T]) {
        log::debug!("Update buffer '{}' with length {}", self.label, data.len());
        if data.len() == self.count {
            application
                .queue
                .write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
        } else {
            self.buffer =
                application
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Instance buffer"),
                        contents: bytemuck::cast_slice(data),
                        usage: self.usage,
                    });
            self.count = data.len();
        }
    }
}

impl<T: Instance + Pod> Buffer<T> {
    // TODO move T to Buffer<T>
    pub fn instance(&self) -> T::Type {
        T::instance(self.handle)
    }
}

impl<T: Uniform + Pod> Buffer<T> {
    // TODO move T to Buffer<T>
    pub fn uniform(&self) -> T::Type {
        T::uniform(self.handle, self.bind_group_layout.clone())
    }
}

impl<'a, T: Pod> InstanceBinding<'a> for Buffer<T> {
    fn handle(&self) -> u64 {
        self.handle
    }
    fn buffer(&'a self) -> &'a wgpu::Buffer {
        &self.buffer
    }
    fn count(&self) -> u32 {
        self.count as u32
    }
    fn bind_group(&'a self) -> &'a wgpu::BindGroup {
        &self.bind_group
    }
}
