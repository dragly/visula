use crate::VertexBufferLayoutBuilder;
use naga::{Expression, Handle};
use std::collections::HashMap;
use std::rc::Rc;
use wgpu::{BindGroup, BindGroupLayout};

pub struct BufferBindingField {
    pub function_argument: u32,
}

pub struct BufferBinding {
    pub slot: u32,
    pub fields: Vec<BufferBindingField>,
    pub layout: VertexBufferLayoutBuilder,
}

pub struct UniformBinding {
    pub expression: Handle<Expression>,
    pub bind_group_layout: Rc<BindGroupLayout>,
}

pub type BindingMap = HashMap<u64, BufferBinding>;
pub type UniformMap = HashMap<u64, UniformBinding>;
pub type BindGroupMap = HashMap<u64, BindGroup>;

pub struct BindingBuilder {
    pub bindings: BindingMap,
    pub uniforms: UniformMap,
    pub bind_groups: BindGroupMap,
    pub shader_location_offset: u32,
    pub entry_point_index: usize,
    pub current_slot: u32,
    pub current_bind_group: u32,
}
