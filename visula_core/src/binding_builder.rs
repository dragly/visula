use crate::error::ShaderError;
use crate::{InstanceBufferInner, TextureBufferInner, UniformBufferInner};
use itertools::Itertools;
use naga::{Expression, Handle};
use naga::{Module, ShaderStage};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wgpu::{
    BindGroup, BindGroupLayout, BufferAddress, VertexAttribute, VertexBufferLayout, VertexStepMode,
};

#[derive(Clone, Debug)]
pub struct VertexBufferLayoutBuilder {
    pub array_stride: BufferAddress,
    pub step_mode: VertexStepMode,
    pub attributes: Vec<VertexAttribute>,
}

impl VertexBufferLayoutBuilder {
    pub fn build(&self) -> VertexBufferLayout<'_> {
        VertexBufferLayout {
            array_stride: self.array_stride,
            step_mode: self.step_mode,
            attributes: &self.attributes,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BufferBindingField {
    pub function_argument: u32,
}

#[derive(Clone, Debug)]
pub struct InstanceBinding {
    pub slot: u32,
    pub fields: Vec<BufferBindingField>,
    pub layout: VertexBufferLayoutBuilder,
    pub inner: Rc<RefCell<InstanceBufferInner>>,
}

pub struct TextureBinding {
    pub inner: Rc<RefCell<TextureBufferInner>>,
}

pub struct UniformBinding {
    pub expression: Handle<Expression>,
    pub bind_group_layout: Rc<BindGroupLayout>,
    pub inner: Rc<RefCell<UniformBufferInner>>,
}

pub type InstanceMap = HashMap<uuid::Uuid, InstanceBinding>;
pub type UniformMap = HashMap<uuid::Uuid, UniformBinding>;
pub type TextureMap = HashMap<uuid::Uuid, TextureBinding>;
pub type BindGroupMap = HashMap<uuid::Uuid, BindGroup>;

pub struct BindingBuilder {
    pub instances: InstanceMap,
    pub uniforms: UniformMap,
    pub textures: TextureMap,
    pub bind_groups: BindGroupMap,
    pub shader_location_offset: u32,
    pub entry_point_index: usize,
    pub current_slot: u32,
    pub current_bind_group: u32,
    pub shader_stage: ShaderStage,
    pub pending_statements: Vec<naga::Statement>,
}

impl BindingBuilder {
    pub fn new(
        module: &Module,
        entry_point_name: &str,
        current_slot: u32,
    ) -> Result<BindingBuilder, ShaderError> {
        log::debug!(
            "Making binding builder for entry point {entry_point_name} and slot {current_slot}"
        );
        let (entry_point_index, entry_point) = module
            .entry_points
            .iter()
            .enumerate()
            .find(|(_index, entry_point)| entry_point.name == entry_point_name)
            .ok_or_else(|| ShaderError::EntryPointNotFound(entry_point_name.to_string()))?;

        let shader_stage = entry_point.stage;

        let shader_location_offset = Self::compute_shader_location_offset(module, entry_point);
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

        Ok(BindingBuilder {
            instances: HashMap::new(),
            uniforms: HashMap::new(),
            textures: HashMap::new(),
            bind_groups: HashMap::new(),
            entry_point_index,
            shader_location_offset,
            current_slot,
            current_bind_group,
            shader_stage,
            pending_statements: Vec::new(),
        })
    }

    fn compute_shader_location_offset(module: &Module, entry_point: &naga::EntryPoint) -> u32 {
        let mut max_location: Option<u32> = None;
        for arg in &entry_point.function.arguments {
            if let Some(naga::Binding::Location { location, .. }) = &arg.binding {
                max_location = Some(max_location.map_or(*location, |m| m.max(*location)));
            } else {
                // Struct argument — check struct members for locations
                let ty = &module.types[arg.ty];
                if let naga::TypeInner::Struct { members, .. } = &ty.inner {
                    for member in members {
                        if let Some(naga::Binding::Location { location, .. }) = &member.binding {
                            max_location =
                                Some(max_location.map_or(*location, |m| m.max(*location)));
                        }
                    }
                }
            }
        }
        max_location.map_or(0, |m| m + 1)
    }

    pub fn sorted_bindings(&self) -> Vec<InstanceBinding> {
        let mut sorted_bindings = self.instances.values().cloned().collect_vec();

        sorted_bindings.sort_by(|a, b| a.slot.cmp(&b.slot));
        sorted_bindings
    }
}
