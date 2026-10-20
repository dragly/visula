use std::{cell::RefCell, rc::Rc};

use crate::{
    binding_builder::{InstanceBinding, UniformBinding, VertexBufferLayoutBuilder},
    instance_buffer::InstanceBufferInner,
    uniform_buffer::UniformBufferInner,
    BindingBuilder, BufferBindingField,
};

#[derive(Clone, Debug)]
pub struct InstanceFieldDescriptor {
    pub name: String,
    pub naga_type: naga::Type,
    pub vertex_attr_format: wgpu::VertexFormat,
}

impl InstanceFieldDescriptor {
    fn vertex_inputs(&self) -> Vec<(String, naga::Type)> {
        match self.naga_type.inner {
            naga::TypeInner::Matrix {
                columns,
                rows,
                scalar,
            } => {
                let column = naga::Type {
                    name: None,
                    inner: naga::TypeInner::Vector { size: rows, scalar },
                };
                ["x_axis", "y_axis", "z_axis", "w_axis"][..columns as usize]
                    .iter()
                    .map(|axis| (format!("{}_{axis}", self.name), column.clone()))
                    .collect()
            }
            _ => vec![(self.name.clone(), self.naga_type.clone())],
        }
    }
}

#[derive(Clone, Debug)]
pub struct InstanceDescriptor {
    pub struct_size: u64,
    pub fields: Vec<InstanceFieldDescriptor>,
}

pub fn integrate_instance(
    descriptor: &InstanceDescriptor,
    inner: &Rc<RefCell<InstanceBufferInner>>,
    handle: &uuid::Uuid,
    module: &mut naga::Module,
    binding_builder: &mut BindingBuilder,
) {
    let entry_point_index = binding_builder.entry_point_index;
    let attributes =
        build_vertex_attributes(&descriptor.fields, binding_builder.shader_location_offset);
    let mut shader_location = binding_builder.shader_location_offset;
    let mut binding_fields = Vec::new();

    for field in &descriptor.fields {
        let mut function_arguments = Vec::new();
        for (name, naga_type) in field.vertex_inputs() {
            let field_type = module.types.insert(naga_type, naga::Span::default());
            module.entry_points[entry_point_index]
                .function
                .arguments
                .push(naga::FunctionArgument {
                    name: Some(name),
                    ty: field_type,
                    binding: Some(naga::Binding::Location {
                        location: shader_location,
                        interpolation: if binding_builder.shader_stage
                            == naga::ShaderStage::Fragment
                        {
                            Some(naga::Interpolation::Flat)
                        } else {
                            None
                        },
                        sampling: None,
                        blend_src: None,
                        per_primitive: false,
                    }),
                });
            function_arguments.push(shader_location);
            shader_location += 1;
        }
        binding_fields.push(BufferBindingField { function_arguments });
    }

    binding_builder.instances.insert(
        *handle,
        InstanceBinding {
            layout: VertexBufferLayoutBuilder {
                array_stride: descriptor.struct_size as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes,
            },
            slot: binding_builder.current_slot,
            fields: binding_fields,
            inner: inner.clone(),
        },
    );

    binding_builder.shader_location_offset = shader_location;
    binding_builder.current_slot += 1;
}

#[derive(Clone, Debug)]
pub struct UniformFieldDescriptor {
    pub name: String,
    pub size: u32,
    pub naga_type: naga::Type,
}

#[derive(Clone, Debug)]
pub struct UniformDescriptor {
    pub struct_name: String,
    pub variable_name: String,
    pub struct_span: u32,
    pub fields: Vec<UniformFieldDescriptor>,
}

pub fn integrate_uniform(
    descriptor: &UniformDescriptor,
    inner: &Rc<RefCell<UniformBufferInner>>,
    handle: &uuid::Uuid,
    module: &mut naga::Module,
    binding_builder: &mut BindingBuilder,
    bind_group_layout: &Rc<wgpu::BindGroupLayout>,
) {
    if binding_builder.uniforms.contains_key(handle) {
        return;
    }

    let entry_point_index = binding_builder.entry_point_index;
    let bind_group = binding_builder.current_bind_group;

    let mut members = Vec::new();
    let mut offset: u32 = 0;

    for field in &descriptor.fields {
        let field_type = module
            .types
            .insert(field.naga_type.clone(), naga::Span::default());
        members.push(naga::StructMember {
            name: Some(field.name.clone()),
            ty: field_type,
            binding: None,
            offset,
        });
        offset += field.size;
    }

    let uniform_type = module.types.insert(
        naga::Type {
            name: Some(descriptor.struct_name.clone()),
            inner: naga::TypeInner::Struct {
                members,
                span: descriptor.struct_span,
            },
        },
        naga::Span::default(),
    );

    let uniform_variable = module.global_variables.append(
        naga::GlobalVariable {
            name: Some(descriptor.variable_name.clone()),
            binding: Some(naga::ResourceBinding {
                group: bind_group,
                binding: 0,
            }),
            space: naga::AddressSpace::Uniform,
            ty: uniform_type,
            init: None,
            memory_decorations: naga::MemoryDecorations::empty(),
        },
        naga::Span::default(),
    );

    let settings_expression = module.entry_points[entry_point_index]
        .function
        .expressions
        .append(
            naga::Expression::GlobalVariable(uniform_variable),
            naga::Span::default(),
        );

    binding_builder.uniforms.insert(
        *handle,
        UniformBinding {
            expression: settings_expression,
            bind_group_layout: bind_group_layout.clone(),
            inner: inner.clone(),
        },
    );
    binding_builder.current_bind_group += 1;
}

pub fn build_vertex_attributes(
    fields: &[InstanceFieldDescriptor],
    shader_location_offset: u32,
) -> Vec<wgpu::VertexAttribute> {
    let mut attributes = Vec::new();
    let mut offset: u64 = 0;

    for field in fields {
        for _ in field.vertex_inputs() {
            attributes.push(wgpu::VertexAttribute {
                format: field.vertex_attr_format,
                offset,
                shader_location: shader_location_offset + attributes.len() as u32,
            });
            offset += field.vertex_attr_format.size();
        }
    }

    attributes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_vertex_attributes() {
        let fields = vec![
            InstanceFieldDescriptor {
                name: "position".to_string(),
                naga_type: naga::Type {
                    name: None,
                    inner: naga::TypeInner::Vector {
                        scalar: naga::Scalar {
                            kind: naga::ScalarKind::Float,
                            width: 4,
                        },
                        size: naga::VectorSize::Tri,
                    },
                },
                vertex_attr_format: wgpu::VertexFormat::Float32x3,
            },
            InstanceFieldDescriptor {
                name: "color".to_string(),
                naga_type: naga::Type {
                    name: None,
                    inner: naga::TypeInner::Vector {
                        scalar: naga::Scalar {
                            kind: naga::ScalarKind::Float,
                            width: 4,
                        },
                        size: naga::VectorSize::Quad,
                    },
                },
                vertex_attr_format: wgpu::VertexFormat::Float32x4,
            },
        ];

        let attrs = build_vertex_attributes(&fields, 5);
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0].shader_location, 5);
        assert_eq!(attrs[0].offset, 0);
        assert_eq!(attrs[0].format, wgpu::VertexFormat::Float32x3);
        assert_eq!(attrs[1].shader_location, 6);
        assert_eq!(attrs[1].offset, 12); // 3 * 4 bytes
        assert_eq!(attrs[1].format, wgpu::VertexFormat::Float32x4);
    }

    #[test]
    fn test_build_vertex_attributes_for_matrix() {
        let fields = vec![
            InstanceFieldDescriptor {
                name: "transform".to_string(),
                naga_type: <glam::Mat4 as crate::NagaType>::naga_type(),
                vertex_attr_format: wgpu::VertexFormat::Float32x4,
            },
            InstanceFieldDescriptor {
                name: "radius".to_string(),
                naga_type: <f32 as crate::NagaType>::naga_type(),
                vertex_attr_format: wgpu::VertexFormat::Float32,
            },
        ];

        let attrs = build_vertex_attributes(&fields, 1);
        let layout: Vec<_> = attrs
            .iter()
            .map(|attr| (attr.shader_location, attr.offset, attr.format))
            .collect();
        assert_eq!(
            layout,
            [
                (1, 0, wgpu::VertexFormat::Float32x4),
                (2, 16, wgpu::VertexFormat::Float32x4),
                (3, 32, wgpu::VertexFormat::Float32x4),
                (4, 48, wgpu::VertexFormat::Float32x4),
                (5, 64, wgpu::VertexFormat::Float32),
            ]
        );
    }
}
