use std::{cell::RefCell, rc::Rc};

use crate::{instance_buffer::InstanceBufferInner, BindingBuilder};

pub trait InstanceHandle {}

pub trait Instance {
    type Type: InstanceHandle;
    fn instance(inner: Rc<RefCell<InstanceBufferInner>>) -> Self::Type;
}

type IntegrateBuffer = fn(
    &Rc<RefCell<InstanceBufferInner>>,
    &uuid::Uuid,
    &mut naga::Module,
    &mut naga::Arena<naga::Expression>,
    &mut BindingBuilder,
);

#[derive(Clone)]
pub struct InstanceField {
    pub buffer_handle: uuid::Uuid,
    pub field_index: usize,
    pub inner: Rc<RefCell<InstanceBufferInner>>,
    pub integrate_buffer: IntegrateBuffer,
}
pub trait InstanceBinding<'a> {
    fn handle(&self) -> uuid::Uuid;
    fn buffer(&'a self) -> &'a wgpu::Buffer;
    fn count(&self) -> u32;
    fn bind_group(&'a self) -> &'a wgpu::BindGroup;
}
