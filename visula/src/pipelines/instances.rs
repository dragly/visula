use std::{cell::RefCell, rc::Rc};
use wgpu::{BindGroupLayout, BufferAddress, VertexAttribute, VertexBufferLayout, VertexStepMode};

use crate::{
    instance_buffer::InstanceBufferInner, uniform_buffer::UniformBufferInner, BindingBuilder,
};

pub struct VertexBufferLayoutBuilder {
    pub array_stride: BufferAddress,
    pub step_mode: VertexStepMode,
    pub attributes: Vec<VertexAttribute>,
}

impl VertexBufferLayoutBuilder {
    pub fn build(&self) -> VertexBufferLayout {
        VertexBufferLayout {
            array_stride: self.array_stride,
            step_mode: self.step_mode,
            attributes: &self.attributes,
        }
    }
}

pub trait InstanceHandle {}

pub trait Instance {
    type Type: InstanceHandle;
    fn instance(inner: Rc<RefCell<InstanceBufferInner>>) -> Self::Type;
}

type IntegrateBuffer =
    fn(&Rc<RefCell<InstanceBufferInner>>, &uuid::Uuid, &mut naga::Module, &mut BindingBuilder);

#[derive(Clone)]
pub struct InstanceField {
    pub buffer_handle: uuid::Uuid,
    pub field_index: usize,
    pub inner: Rc<RefCell<InstanceBufferInner>>,
    pub integrate_buffer: IntegrateBuffer,
}

pub trait Uniform {
    type Type;
    fn uniform(inner: Rc<RefCell<UniformBufferInner>>) -> Self::Type;
}

type IntegrateUniform = fn(
    &Rc<RefCell<UniformBufferInner>>,
    &uuid::Uuid,
    &mut naga::Module,
    &mut BindingBuilder,
    &Rc<BindGroupLayout>,
);

#[derive(Clone)]
pub struct UniformField {
    pub bind_group_layout: std::rc::Rc<wgpu::BindGroupLayout>,
    pub buffer_handle: uuid::Uuid,
    pub field_index: usize,
    pub inner: Rc<RefCell<UniformBufferInner>>,
    pub integrate_buffer: IntegrateUniform,
}

pub trait UniformHandle {}
