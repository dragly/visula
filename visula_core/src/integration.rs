use std::{cell::RefCell, rc::Rc};

use wgpu::BindGroupLayout;

use crate::{
    BindingBuilder, BufferBindingField, InstanceBinding, InstanceBufferInner, UniformBinding,
    UniformBufferInner, VertexBufferLayoutBuilder,
};

pub struct InstanceFieldDescriptor {
    pub name: String,
    pub naga_type: naga::Type,
    pub vertex_attr_format: wgpu::VertexFormat,
    pub size: u64,
}

pub struct UniformFieldDescriptor {
    pub name: String,
    pub naga_type: naga::Type,
    pub size: u32,
}

pub fn integrate_instance(
    fields: &[InstanceFieldDescriptor],
    array_stride: wgpu::BufferAddress,
    inner: &Rc<RefCell<InstanceBufferInner>>,
    handle: &uuid::Uuid,
    module: &mut naga::Module,
    binding_builder: &mut BindingBuilder,
) {
    let entry_point_index = binding_builder.entry_point_index;
    let previous_shader_location_offset = binding_builder.shader_location_offset;

    let mut attributes = Vec::new();
    let mut binding_fields = Vec::new();
    let mut offset: u64 = 0;

    for (i, field) in fields.iter().enumerate() {
        let field_type = module
            .types
            .insert(field.naga_type.clone(), naga::Span::default());
        module.entry_points[entry_point_index]
            .function
            .arguments
            .push(naga::FunctionArgument {
                name: Some(field.name.clone()),
                ty: field_type,
                binding: Some(naga::Binding::Location {
                    location: previous_shader_location_offset + i as u32,
                    interpolation: None,
                    sampling: None,
                    blend_src: None,
                }),
            });

        attributes.push(wgpu::VertexAttribute {
            format: field.vertex_attr_format,
            offset,
            shader_location: previous_shader_location_offset + i as u32,
        });

        binding_fields.push(BufferBindingField {
            function_argument: previous_shader_location_offset + i as u32,
        });

        offset += field.size;
    }

    binding_builder.instances.insert(
        *handle,
        InstanceBinding {
            layout: VertexBufferLayoutBuilder {
                array_stride,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes,
            },
            slot: binding_builder.current_slot,
            fields: binding_fields,
            inner: inner.clone(),
        },
    );

    binding_builder.shader_location_offset += fields.len() as u32;
    binding_builder.current_slot += 1;
}

pub struct IntegrateUniformParams<'a> {
    pub struct_name: &'a str,
    pub variable_name: &'a str,
    pub fields: &'a [UniformFieldDescriptor],
    pub struct_span: u32,
    pub inner: &'a Rc<RefCell<UniformBufferInner>>,
    pub handle: &'a uuid::Uuid,
    pub bind_group_layout: &'a Rc<BindGroupLayout>,
}

pub fn integrate_uniform(
    params: &IntegrateUniformParams,
    module: &mut naga::Module,
    binding_builder: &mut BindingBuilder,
) {
    let IntegrateUniformParams {
        struct_name,
        variable_name,
        fields,
        struct_span,
        inner,
        handle,
        bind_group_layout,
    } = params;

    if binding_builder.uniforms.contains_key(handle) {
        return;
    }

    let entry_point_index = binding_builder.entry_point_index;
    let bind_group = binding_builder.current_bind_group;

    let mut members = Vec::new();
    let mut offset: u32 = 0;

    for field in *fields {
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
            name: Some((*struct_name).into()),
            inner: naga::TypeInner::Struct {
                members,
                span: *struct_span,
            },
        },
        naga::Span::default(),
    );

    let uniform_variable = module.global_variables.append(
        naga::GlobalVariable {
            name: Some((*variable_name).into()),
            binding: Some(naga::ResourceBinding {
                group: bind_group,
                binding: 0,
            }),
            space: naga::AddressSpace::Uniform,
            ty: uniform_type,
            init: None,
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
        **handle,
        UniformBinding {
            expression: settings_expression,
            bind_group_layout: (*bind_group_layout).clone(),
            inner: (*inner).clone(),
        },
    );
    binding_builder.current_bind_group += 1;
}

pub fn compute_vertex_attributes(
    fields: &[(wgpu::VertexFormat, u64)],
    shader_location_offset: u32,
) -> Vec<wgpu::VertexAttribute> {
    let mut attributes = Vec::new();
    let mut offset: u64 = 0;

    for (i, (format, size)) in fields.iter().enumerate() {
        attributes.push(wgpu::VertexAttribute {
            format: *format,
            offset,
            shader_location: shader_location_offset + i as u32,
        });
        offset += size;
    }

    attributes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_vertex_attributes_empty() {
        let attrs = compute_vertex_attributes(&[], 0);
        assert!(attrs.is_empty());
    }

    #[test]
    fn test_compute_vertex_attributes_single_field() {
        let fields = vec![(wgpu::VertexFormat::Float32x3, 12)];
        let attrs = compute_vertex_attributes(&fields, 0);
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].format, wgpu::VertexFormat::Float32x3);
        assert_eq!(attrs[0].offset, 0);
        assert_eq!(attrs[0].shader_location, 0);
    }

    #[test]
    fn test_compute_vertex_attributes_multiple_fields() {
        let fields = vec![
            (wgpu::VertexFormat::Float32x3, 12), // vec3
            (wgpu::VertexFormat::Float32x4, 16), // vec4
            (wgpu::VertexFormat::Float32, 4),     // f32
        ];
        let attrs = compute_vertex_attributes(&fields, 5);
        assert_eq!(attrs.len(), 3);

        assert_eq!(attrs[0].format, wgpu::VertexFormat::Float32x3);
        assert_eq!(attrs[0].offset, 0);
        assert_eq!(attrs[0].shader_location, 5);

        assert_eq!(attrs[1].format, wgpu::VertexFormat::Float32x4);
        assert_eq!(attrs[1].offset, 12);
        assert_eq!(attrs[1].shader_location, 6);

        assert_eq!(attrs[2].format, wgpu::VertexFormat::Float32);
        assert_eq!(attrs[2].offset, 28);
        assert_eq!(attrs[2].shader_location, 7);
    }

    #[test]
    fn test_compute_vertex_attributes_with_offset() {
        let fields = vec![(wgpu::VertexFormat::Float32x2, 8)];
        let attrs = compute_vertex_attributes(&fields, 10);
        assert_eq!(attrs[0].shader_location, 10);
    }

    fn make_test_module() -> naga::Module {
        let mut module = naga::Module::default();
        module.entry_points.push(naga::EntryPoint {
            name: "vs_main".into(),
            stage: naga::ShaderStage::Vertex,
            early_depth_test: None,
            workgroup_size: [0; 3],
            workgroup_size_overrides: None,
            function: naga::Function::default(),
        });
        module
    }

    #[test]
    fn test_integrate_instance_empty_fields() {
        let mut module = make_test_module();
        let inner = Rc::new(RefCell::new(InstanceBufferInner::new_for_testing(
            "test_instance",
        )));
        let handle = inner.borrow().handle;
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 0);

        integrate_instance(&[], 0, &inner, &handle, &mut module, &mut binding_builder);

        assert_eq!(binding_builder.instances.len(), 1);
        let binding = binding_builder.instances.get(&handle).unwrap();
        assert!(binding.layout.attributes.is_empty());
        assert!(binding.fields.is_empty());
        assert_eq!(binding_builder.shader_location_offset, 0);
        assert_eq!(binding_builder.current_slot, 1);
    }

    #[test]
    fn test_integrate_instance_single_field() {
        let mut module = make_test_module();
        let inner = Rc::new(RefCell::new(InstanceBufferInner::new_for_testing(
            "test_instance",
        )));
        let handle = inner.borrow().handle;
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 0);

        let fields = vec![InstanceFieldDescriptor {
            name: "position".into(),
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
            size: 12,
        }];

        integrate_instance(
            &fields,
            12,
            &inner,
            &handle,
            &mut module,
            &mut binding_builder,
        );

        assert_eq!(binding_builder.instances.len(), 1);
        let binding = binding_builder.instances.get(&handle).unwrap();
        assert_eq!(binding.layout.attributes.len(), 1);
        assert_eq!(
            binding.layout.attributes[0].format,
            wgpu::VertexFormat::Float32x3
        );
        assert_eq!(binding.layout.attributes[0].offset, 0);
        assert_eq!(binding.layout.attributes[0].shader_location, 0);
        assert_eq!(binding.layout.array_stride, 12);
        assert_eq!(binding.fields.len(), 1);

        assert_eq!(
            module.entry_points[0].function.arguments.len(),
            1,
            "should have added one function argument"
        );
        assert_eq!(
            module.entry_points[0].function.arguments[0]
                .name
                .as_deref(),
            Some("position")
        );

        assert_eq!(binding_builder.shader_location_offset, 1);
        assert_eq!(binding_builder.current_slot, 1);
    }

    #[test]
    fn test_integrate_instance_multiple_fields() {
        let mut module = make_test_module();
        let inner = Rc::new(RefCell::new(InstanceBufferInner::new_for_testing(
            "test_instance",
        )));
        let handle = inner.borrow().handle;
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 0);

        let fields = vec![
            InstanceFieldDescriptor {
                name: "position".into(),
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
                size: 12,
            },
            InstanceFieldDescriptor {
                name: "color".into(),
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
                size: 16,
            },
        ];

        integrate_instance(
            &fields,
            28,
            &inner,
            &handle,
            &mut module,
            &mut binding_builder,
        );

        let binding = binding_builder.instances.get(&handle).unwrap();
        assert_eq!(binding.layout.attributes.len(), 2);
        assert_eq!(binding.layout.attributes[0].offset, 0);
        assert_eq!(binding.layout.attributes[1].offset, 12);
        assert_eq!(binding.layout.attributes[0].shader_location, 0);
        assert_eq!(binding.layout.attributes[1].shader_location, 1);
        assert_eq!(binding_builder.shader_location_offset, 2);
    }

    #[test]
    fn test_integrate_instance_respects_existing_shader_locations() {
        let mut module = make_test_module();
        let pre_type = module.types.insert(
            naga::Type {
                name: None,
                inner: naga::TypeInner::Vector {
                    scalar: naga::Scalar {
                        kind: naga::ScalarKind::Float,
                        width: 4,
                    },
                    size: naga::VectorSize::Tri,
                },
            },
            naga::Span::default(),
        );
        module.entry_points[0]
            .function
            .arguments
            .push(naga::FunctionArgument {
                name: Some("existing".into()),
                ty: pre_type,
                binding: Some(naga::Binding::Location {
                    location: 0,
                    interpolation: None,
                    sampling: None,
                    blend_src: None,
                }),
            });

        let inner = Rc::new(RefCell::new(InstanceBufferInner::new_for_testing(
            "test_instance",
        )));
        let handle = inner.borrow().handle;
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 0);

        let fields = vec![InstanceFieldDescriptor {
            name: "value".into(),
            naga_type: naga::Type {
                name: None,
                inner: naga::TypeInner::Scalar(naga::Scalar {
                    kind: naga::ScalarKind::Float,
                    width: 4,
                }),
            },
            vertex_attr_format: wgpu::VertexFormat::Float32,
            size: 4,
        }];

        integrate_instance(
            &fields,
            4,
            &inner,
            &handle,
            &mut module,
            &mut binding_builder,
        );

        let binding = binding_builder.instances.get(&handle).unwrap();
        assert_eq!(
            binding.layout.attributes[0].shader_location, 1,
            "should start after existing arguments"
        );
        assert_eq!(binding_builder.shader_location_offset, 2);
    }

    fn make_uniform_params<'a>(
        fields: &'a [UniformFieldDescriptor],
        struct_span: u32,
        inner: &'a Rc<RefCell<UniformBufferInner>>,
        handle: &'a uuid::Uuid,
        bind_group_layout: &'a Rc<wgpu::BindGroupLayout>,
    ) -> IntegrateUniformParams<'a> {
        IntegrateUniformParams {
            struct_name: "TestUniform",
            variable_name: "test",
            fields,
            struct_span,
            inner,
            handle,
            bind_group_layout,
        }
    }

    #[test]
    fn test_integrate_uniform_single_field() {
        let mut module = make_test_module();
        let inner = Rc::new(RefCell::new(UniformBufferInner::new_for_testing(
            "test_uniform",
        )));
        let handle = inner.borrow().handle;
        let bind_group_layout = inner.borrow().bind_group_layout.clone();
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 0);

        let fields = vec![UniformFieldDescriptor {
            name: "scale".into(),
            naga_type: naga::Type {
                name: None,
                inner: naga::TypeInner::Scalar(naga::Scalar {
                    kind: naga::ScalarKind::Float,
                    width: 4,
                }),
            },
            size: 4,
        }];

        let params = make_uniform_params(&fields, 4, &inner, &handle, &bind_group_layout);
        integrate_uniform(&params, &mut module, &mut binding_builder);

        assert_eq!(binding_builder.uniforms.len(), 1);
        assert!(binding_builder.uniforms.contains_key(&handle));
        assert_eq!(binding_builder.current_bind_group, 2);
        assert_eq!(module.global_variables.len(), 1);
    }

    #[test]
    fn test_integrate_uniform_skips_duplicate() {
        let mut module = make_test_module();
        let inner = Rc::new(RefCell::new(UniformBufferInner::new_for_testing(
            "test_uniform",
        )));
        let handle = inner.borrow().handle;
        let bind_group_layout = inner.borrow().bind_group_layout.clone();
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 0);

        let fields = vec![UniformFieldDescriptor {
            name: "scale".into(),
            naga_type: naga::Type {
                name: None,
                inner: naga::TypeInner::Scalar(naga::Scalar {
                    kind: naga::ScalarKind::Float,
                    width: 4,
                }),
            },
            size: 4,
        }];

        let params = make_uniform_params(&fields, 4, &inner, &handle, &bind_group_layout);
        integrate_uniform(&params, &mut module, &mut binding_builder);
        let bind_group_after_first = binding_builder.current_bind_group;

        integrate_uniform(&params, &mut module, &mut binding_builder);

        assert_eq!(binding_builder.uniforms.len(), 1);
        assert_eq!(binding_builder.current_bind_group, bind_group_after_first);
        assert_eq!(module.global_variables.len(), 1);
    }

    #[test]
    fn test_integrate_uniform_multiple_fields() {
        let mut module = make_test_module();
        let inner = Rc::new(RefCell::new(UniformBufferInner::new_for_testing(
            "test_uniform",
        )));
        let handle = inner.borrow().handle;
        let bind_group_layout = inner.borrow().bind_group_layout.clone();
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 0);

        let fields = vec![
            UniformFieldDescriptor {
                name: "scale".into(),
                naga_type: naga::Type {
                    name: None,
                    inner: naga::TypeInner::Scalar(naga::Scalar {
                        kind: naga::ScalarKind::Float,
                        width: 4,
                    }),
                },
                size: 4,
            },
            UniformFieldDescriptor {
                name: "offset".into(),
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
                size: 12,
            },
        ];

        let params = make_uniform_params(&fields, 16, &inner, &handle, &bind_group_layout);
        integrate_uniform(&params, &mut module, &mut binding_builder);

        assert_eq!(binding_builder.uniforms.len(), 1);
        let global_var = module.global_variables.iter().next().unwrap().1;
        let ty = &module.types[global_var.ty];
        match &ty.inner {
            naga::TypeInner::Struct { members, span } => {
                assert_eq!(members.len(), 2);
                assert_eq!(members[0].name.as_deref(), Some("scale"));
                assert_eq!(members[0].offset, 0);
                assert_eq!(members[1].name.as_deref(), Some("offset"));
                assert_eq!(members[1].offset, 4);
                assert_eq!(*span, 16);
            }
            _ => panic!("expected struct type"),
        }
    }
}
