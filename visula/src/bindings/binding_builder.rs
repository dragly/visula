use crate::{BufferInner, VertexBufferLayoutBuilder};
use naga::Module;
use naga::{Expression, Handle};
use std::cell::RefCell;
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
    pub inner: Rc<RefCell<BufferInner>>,
}

pub struct UniformBinding {
    pub expression: Handle<Expression>,
    pub bind_group_layout: Rc<BindGroupLayout>,
    pub inner: Rc<RefCell<BufferInner>>,
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

impl BindingBuilder {
    pub fn new(module: &Module, entry_point_name: &str, current_slot: u32) -> BindingBuilder {
        log::debug!(
            "Making binding builder for entry point {entry_point_name} and slot {current_slot}"
        );
        let (entry_point_index, entry_point) = module
            .entry_points
            .iter()
            .enumerate()
            .find(|(_index, entry_point)| entry_point.name == entry_point_name)
            .unwrap();

        let shader_location_offset = entry_point.function.arguments.len() as u32;
        log::debug!("shader_location_offset: {shader_location_offset}");

        let current_bind_group = 1 + module
            .global_variables
            .iter()
            .map(|(_handle, item)| item.binding.as_ref().map(|b| b.group))
            .fold(0, |accum, binding| {
                if let Some(current) = binding {
                    if current > accum {
                        current
                    } else {
                        accum
                    }
                } else {
                    accum
                }
            });
        log::debug!("current_bind_group: {current_bind_group}");

        BindingBuilder {
            bindings: HashMap::new(),
            uniforms: HashMap::new(),
            bind_groups: HashMap::new(),
            entry_point_index,
            shader_location_offset,
            current_slot,
            current_bind_group,
        }
    }
}
