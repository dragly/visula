use std::{cell::RefCell, rc::Rc};
use wgpu::BindGroupLayout;

use crate::{uniform_buffer::UniformBufferInner, BindingBuilder};
pub trait Uniform {
    type Type;
    fn uniform(inner: Rc<RefCell<UniformBufferInner>>) -> Self::Type;
}

type IntegrateUniform = Rc<RefCell<dyn Fn(
    &Rc<RefCell<UniformBufferInner>>,
    &uuid::Uuid,
    &mut naga::Module,
    &mut BindingBuilder,
    &Rc<BindGroupLayout>,
)>>;

#[derive(Clone)]
pub struct UniformField {
    pub bind_group_layout: Rc<wgpu::BindGroupLayout>,
    pub buffer_handle: uuid::Uuid,
    pub field_index: usize,
    pub inner: Rc<RefCell<UniformBufferInner>>,
    pub integrate_buffer: IntegrateUniform,
}

pub trait UniformHandle {}
