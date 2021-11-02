use naga::{Expression, Handle, Module, Span};
use std::rc::Rc;
use wgpu::{BindGroupLayout, BufferAddress, VertexAttribute, VertexBufferLayout, VertexStepMode};

use crate::BindingBuilder;

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

pub trait InstanceHandle {
    fn integrate(&self, module: &mut Module, binding_builder: &mut BindingBuilder);
}

pub trait Instance {
    type Type: InstanceHandle;
    fn instance(handle: u64) -> Self::Type;
}

pub struct InstanceField {
    pub buffer_handle: u64,
    pub field_index: usize,
}

impl InstanceField {
    pub fn integrate(
        &self,
        module: &mut Module,
        binding_builder: &BindingBuilder,
    ) -> Handle<Expression> {
        module.entry_points[binding_builder.entry_point_index]
            .function
            .expressions
            .append(
                Expression::FunctionArgument(
                    binding_builder.bindings[&self.buffer_handle].fields[self.field_index]
                        .function_argument,
                ),
                Span::Unknown,
            )
    }
}

pub trait Uniform {
    type Type;
    fn uniform(handle: u64, bind_group_layout: Rc<BindGroupLayout>) -> Self::Type;
}

pub struct UniformField {
    pub buffer_handle: u64,
    pub field_index: usize,
}

pub trait UniformHandle {
    fn integrate(&self, module: &mut Module, binding_builder: &mut BindingBuilder);
}

impl UniformField {
    pub fn integrate(
        &self,
        module: &mut Module,
        binding_builder: &BindingBuilder,
    ) -> Handle<Expression> {
        module.entry_points[binding_builder.entry_point_index]
            .function
            .expressions
            .append(
                Expression::AccessIndex {
                    index: self.field_index as u32,
                    base: binding_builder.uniforms[&self.buffer_handle].expression,
                },
                Span::Unknown,
            )
    }
}
