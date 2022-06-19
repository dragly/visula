use naga::{Expression, Handle, Module, Span};
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

type IntegrateBuffer = fn(&Rc<RefCell<BufferInner>>, u64, &mut naga::Module, &mut BindingBuilder);

pub struct InstanceField {
    pub buffer_handle: u64,
    pub field_index: usize,
    pub inner: Rc<RefCell<BufferInner>>,
    pub integrate_buffer: IntegrateBuffer,
}

impl InstanceField {
    pub fn integrate(
        &self,
        module: &mut Module,
        binding_builder: &mut BindingBuilder,
    ) -> Handle<Expression> {
        if !binding_builder.bindings.contains_key(&self.buffer_handle) {
            (self.integrate_buffer)(&self.inner, self.buffer_handle, module, binding_builder);
        }
        module.entry_points[binding_builder.entry_point_index]
            .function
            .expressions
            .append(
                Expression::FunctionArgument(
                    binding_builder.bindings[&self.buffer_handle].fields[self.field_index]
                        .function_argument,
                ),
                Span::default(),
            )
    }
}

pub trait Uniform {
    type Type;
    fn uniform(inner: Rc<RefCell<BufferInner>>) -> Self::Type;
}

type IntegrateUniform = fn(
    &Rc<RefCell<BufferInner>>,
    u64,
    &mut naga::Module,
    &mut BindingBuilder,
    &Rc<BindGroupLayout>,
);

pub struct UniformField {
    pub bind_group_layout: std::rc::Rc<wgpu::BindGroupLayout>,
    pub buffer_handle: u64,
    pub field_index: usize,
    pub inner: Rc<RefCell<BufferInner>>,
    pub integrate_buffer: IntegrateUniform,
}

pub trait UniformHandle {}

impl UniformField {
    pub fn integrate(
        &self,
        module: &mut Module,
        binding_builder: &mut BindingBuilder,
    ) -> Handle<Expression> {
        let inner = self.inner.borrow();
        if !binding_builder.bindings.contains_key(&self.buffer_handle) {
            (self.integrate_buffer)(
                &self.inner,
                self.buffer_handle,
                module,
                binding_builder,
                &inner.bind_group_layout,
            );
        }
        let access_index = module.entry_points[binding_builder.entry_point_index]
            .function
            .expressions
            .append(
                Expression::AccessIndex {
                    index: self.field_index as u32,
                    base: binding_builder.uniforms[&self.buffer_handle].expression,
                },
                Span::default(),
            );
        module.entry_points[binding_builder.entry_point_index]
            .function
            .expressions
            .append(
                Expression::Load {
                    pointer: access_index,
                },
                Span::default(),
            )
    }
}
