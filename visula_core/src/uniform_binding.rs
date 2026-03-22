use std::{cell::RefCell, rc::Rc};

use crate::{integrate::UniformDescriptor, uniform_buffer::UniformBufferInner};

pub trait Uniform {
    type Type;
    fn uniform(inner: Rc<RefCell<UniformBufferInner>>) -> Self::Type;
}

#[derive(Clone)]
pub struct UniformField {
    pub bind_group_layout: Rc<wgpu::BindGroupLayout>,
    pub buffer_handle: uuid::Uuid,
    pub field_index: usize,
    pub inner: Rc<RefCell<UniformBufferInner>>,
    pub descriptor: Rc<UniformDescriptor>,
}

pub trait UniformHandle {}
