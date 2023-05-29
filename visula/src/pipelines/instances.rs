use std::{cell::RefCell, rc::Rc};
use wgpu::{BindGroupLayout, BufferAddress, VertexAttribute, VertexBufferLayout, VertexStepMode};

use crate::{BindingBuilder, BufferInner};

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
    fn instance(inner: Rc<RefCell<BufferInner>>) -> Self::Type;
}

type IntegrateBuffer =
    fn(&Rc<RefCell<BufferInner>>, &uuid::Uuid, &mut naga::Module, &mut BindingBuilder);

#[derive(Clone)]
pub struct InstanceField {
    pub buffer_handle: uuid::Uuid,
    pub field_index: usize,
    pub inner: Rc<RefCell<BufferInner>>,
    pub integrate_buffer: IntegrateBuffer,
}

pub trait Uniform {
    type Type;
    fn uniform(inner: Rc<RefCell<BufferInner>>) -> Self::Type;
}

type IntegrateUniform = fn(
    &Rc<RefCell<BufferInner>>,
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
    pub inner: Rc<RefCell<BufferInner>>,
    pub integrate_buffer: IntegrateUniform,
}

pub trait UniformHandle {}
