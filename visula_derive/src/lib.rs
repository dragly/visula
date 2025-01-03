use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, FieldsNamed, ItemStruct};

type TokenStream2 = proc_macro2::TokenStream;

#[proc_macro_derive(Delegate)]
pub fn delegate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::Item);
    let result = match input {
        syn::Item::Struct(ItemStruct {
            ident: struct_ident,
            fields,
            ..
        }) => {
            let mut field_insertions: Vec<TokenStream2> = Vec::new();
            let mut pyo3_fields: Vec<TokenStream2> = Vec::new();
            let mut pyo3_attributes: Vec<TokenStream2> = Vec::new();
            let mut pyo3_field_names: Vec<TokenStream2> = Vec::new();
            match fields {
                Fields::Named(FieldsNamed { named, .. }) => {
                    for field in named.iter() {
                        let Field { ident, .. } = field;
                        field_insertions.push(quote! {
                            self.#ident.clone(),
                        });

                        pyo3_fields.push(quote! {
                            #[pyo3(get, set)]
                            pub #ident : ::pyo3::PyObject,
                        });
                        pyo3_attributes.push(quote! {
                            #ident : ::pyo3::PyObject,
                        });
                        pyo3_field_names.push(quote! {
                            #ident,
                        });
                    }
                }
                _ => unimplemented!(),
            };
            let pyclass_struct_name = format_ident!("Py{}", struct_ident);
            let pyclass_attribute: TokenStream2 = format!(
                "#[::pyo3::pyclass(name = \"{}\", unsendable)]",
                struct_ident
            )
            .parse()
            .unwrap();
            quote! {
                #[cfg(not(target_arch = "wasm32"))]
                #pyclass_attribute
                #[derive(Clone)]
                pub struct #pyclass_struct_name {
                    #(#pyo3_fields)*
                }


                #[cfg(not(target_arch = "wasm32"))]
                #[::pyo3::pymethods]
                impl #pyclass_struct_name {
                    #[new]
                    fn new(py: ::pyo3::Python, #(#pyo3_attributes)*) -> Self {
                        Self {
                            #(#pyo3_field_names)*
                        }
                    }
                }

                impl #struct_ident {
                    fn inject(&self, shader_variable_name: &str, module: &mut ::naga::Module, binding_builder: &mut BindingBuilder) {
                        let fields = vec![
                            #(#field_insertions)*
                        ];
                        ::visula_core::inject::inject(module, binding_builder, ::visula_core::naga::ShaderStage::Vertex, shader_variable_name, &fields);
                    }
                }
            }
        }
        _ => unimplemented!(),
    };
    TokenStream::from(result)
}

#[proc_macro_derive(Instance)]
pub fn instance(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let mut attributes = Vec::new();
    let mut sizes = Vec::new();
    let mut shader_location: u32 = 0;
    let mut field_index: usize = 0;
    let mut instance_struct_fields = Vec::new();
    let mut module_fields = Vec::new();
    let mut instance_field_values = Vec::new();
    let mut binding_fields = Vec::new();

    let instance_struct_name = format_ident!("{}Instance", name);

    match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                for field in &fields.named {
                    let field_name = &field.ident;
                    match field_name {
                        Some(field_name) => {
                            let field_type = &field.ty;
                            let size = quote! {
                                (std::mem::size_of::<#field_type>() as u64)
                            };
                            let offset = quote! {
                                0 #(+ #sizes)*
                            };
                            let format = quote! {
                                < #field_type as visula_core::VertexAttrFormat >::vertex_attr_format()
                            };
                            instance_struct_fields.push(quote! {
                                pub #field_name: visula_core::Expression
                            });
                            let naga_type = quote! {
                                < #field_type as visula_core::NagaType >::naga_type()
                            };
                            module_fields.push(quote! {
                                {
                                    let field_type = module.types.insert(#naga_type, ::visula_core::naga::Span::default());
                                    module.entry_points[entry_point_index]
                                        .function
                                        .arguments
                                        .push(::visula_core::naga::FunctionArgument {
                                            name: Some(stringify!(#field_name).into()),
                                            ty: field_type,
                                            binding: Some(::visula_core::naga::Binding::Location {
                                                location: previous_shader_location_offset + #shader_location,
                                                interpolation: None,
                                                sampling: None,
                                            }),
                                        });
                                }
                            });
                            instance_field_values.push(quote! {
                                #field_name: visula_core::Expression::InstanceField(visula_core::InstanceField {
                                    buffer_handle: inner.borrow().handle,
                                    inner: inner.clone(),
                                    field_index: #field_index,
                                    integrate_buffer: #instance_struct_name::integrate,
                                })
                            });
                            attributes.push(quote! {
                                ::visula_core::wgpu::VertexAttribute{
                                    format: #format,
                                    offset: #offset as u64,
                                    shader_location: previous_shader_location_offset + #shader_location,
                                }
                            });
                            binding_fields.push(quote! {
                                ::visula_core::BufferBindingField {
                                    function_argument: previous_shader_location_offset + #shader_location,
                                }
                            });
                            sizes.push(size);
                            shader_location += 1;
                            field_index += 1;
                        }
                        None => unimplemented!(),
                    }
                }
            }
            Fields::Unnamed(_) | Fields::Unit => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }

    let expanded = quote! {
        pub struct #instance_struct_name {
            #(#instance_struct_fields,)*
            pub handle: ::visula_core::uuid::Uuid,
        }

        impl #instance_struct_name {
            fn integrate(
                inner: &std::rc::Rc<std::cell::RefCell<visula_core::InstanceBufferInner>>,
                handle: &::visula_core::uuid::Uuid,
                module: &mut ::visula_core::naga::Module,
                binding_builder: &mut visula_core::BindingBuilder,
            )
            {
                let entry_point_index = binding_builder.entry_point_index;
                let previous_shader_location_offset = binding_builder.shader_location_offset;
                let slot = binding_builder.current_slot;

                #(#module_fields)*

                binding_builder.bindings.insert(handle.clone(), visula_core::BufferBinding {
                    layout: visula_core::VertexBufferLayoutBuilder {
                        array_stride: std::mem::size_of::<#name>() as ::visula_core::wgpu::BufferAddress,
                        step_mode: ::visula_core::wgpu::VertexStepMode::Instance,
                        attributes: vec![
                            #(#attributes),*
                        ],
                    },
                    slot: binding_builder.current_slot,
                    fields: vec![
                        #(#binding_fields,)*
                    ],
                    inner: inner.clone(),
                });

                binding_builder.shader_location_offset += #shader_location;
                binding_builder.current_slot += 1;
            }

        }

        impl visula_core::InstanceHandle for #instance_struct_name {
        }

        impl visula_core::Instance for #name {
            type Type = #instance_struct_name;
            fn instance( inner: std::rc::Rc<std::cell::RefCell<visula_core::InstanceBufferInner>>) -> Self::Type {
                let handle = inner.borrow().handle;
                Self::Type {
                    #(#instance_field_values,)*
                    handle,
                }
            }
        }

    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(Uniform)]
pub fn uniform(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let mut sizes = Vec::new();
    let mut uniform_struct_fields = Vec::new();
    let mut uniform_field_types_init = Vec::new();
    let mut uniform_fields = Vec::new();
    let mut uniform_field_values = Vec::new();
    let mut field_index: usize = 0;

    let uniform_struct_name = format_ident!("{}Uniform", name);

    match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                for field in &fields.named {
                    let field_name = &field.ident;
                    match field_name {
                        Some(field_name) => {
                            let field_type = &field.ty;
                            uniform_struct_fields.push(quote! {
                                #field_name: visula_core::Expression
                            });
                            let size = quote! {
                                (std::mem::size_of::<#field_type>() as u32)
                            };
                            // TODO figure out why this cannot be visula_core and needs to be
                            // visula
                            let naga_type = quote! {
                                < #field_type as visula_core::NagaType >::naga_type()
                            };
                            let field_type_declaration = format_ident!("{}_type", field_name);
                            uniform_field_types_init.push(quote! {
                                let #field_type_declaration = module.types.insert(#naga_type, ::visula_core::naga::Span::default());
                            });
                            let offset = quote! {
                                0 #(+ #sizes)*
                            };
                            uniform_fields.push(quote! {
                                ::visula_core::naga::StructMember {
                                    name: Some(stringify!(#field_name).into()),
                                    ty: #field_type_declaration,
                                    binding: None,
                                    offset: #offset,
                                }
                            });
                            uniform_field_values.push(quote! {
                                #field_name: visula_core::Expression::UniformField(visula_core::UniformField {
                                    buffer_handle: inner.borrow().handle,
                                    inner: inner.clone(),
                                    field_index: #field_index,
                                    bind_group_layout: inner.borrow().bind_group_layout.clone(),
                                    integrate_buffer: ::std::rc::Rc::new(::std::cell::RefCell::new(#uniform_struct_name::integrate)),
                                })
                            });
                            field_index += 1;
                            sizes.push(size);
                        }
                        None => unimplemented!(),
                    }
                }
            }
            Fields::Unnamed(_) | Fields::Unit => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }

    let expanded = quote! {
        struct #uniform_struct_name {
            #(#uniform_struct_fields,)*
            handle: ::visula_core::uuid::Uuid,
            bind_group_layout: std::rc::Rc<::visula_core::wgpu::BindGroupLayout>,
        }

        impl visula_core::UniformHandle for #uniform_struct_name {
        }
        impl #uniform_struct_name {
            fn integrate(
                inner: &std::rc::Rc<std::cell::RefCell<visula_core::UniformBufferInner>>,
                handle: &::visula_core::uuid::Uuid,
                module: &mut ::visula_core::naga::Module,
                binding_builder: &mut visula_core::BindingBuilder,
                bind_group_layout: &std::rc::Rc<::visula_core::wgpu::BindGroupLayout>,
            )
            {
                if binding_builder.uniforms.contains_key(&handle.clone()) {
                    return;
                };

                let entry_point_index = binding_builder.entry_point_index;
                let previous_shader_location_offset = binding_builder.shader_location_offset;
                let slot = binding_builder.current_slot;
                let bind_group = binding_builder.current_bind_group;

                #(#uniform_field_types_init)*

                let uniform_type = module.types.insert(
                    ::visula_core::naga::Type {
                        name: Some(stringify!(#uniform_struct_name).into()),
                        inner: ::visula_core::naga::TypeInner::Struct {
                            members: vec![
                                #(#uniform_fields),*
                            ],
                            span: ::std::mem::size_of::<#uniform_struct_name>() as u32,
                        },
                    },
                    ::visula_core::naga::Span::default(),
                );
                let uniform_variable = module.global_variables.append(
                    ::visula_core::naga::GlobalVariable {
                        name: Some(stringify!(#name).to_lowercase().into()),
                        binding: Some(::visula_core::naga::ResourceBinding {
                            group: bind_group,
                            binding: 0,
                        }),
                        space: ::visula_core::naga::AddressSpace::Uniform,
                        ty: uniform_type,
                        init: None,
                    },
                    ::visula_core::naga::Span::default(),
                );
                let settings_expression = module.entry_points[entry_point_index]
                    .function
                    .expressions
                    .append(::visula_core::naga::Expression::GlobalVariable(uniform_variable), ::visula_core::naga::Span::default());

                binding_builder.uniforms.insert(handle.clone(), visula_core::UniformBinding {
                    expression: settings_expression,
                    bind_group_layout: bind_group_layout.clone(),
                    inner: inner.clone(),
                });
                binding_builder.current_bind_group += 1;
            }

        }

        impl visula_core::Uniform for #name {
            type Type = #uniform_struct_name;
            fn uniform( inner: std::rc::Rc<std::cell::RefCell<visula_core::UniformBufferInner>>) -> Self::Type {
                Self::Type {
                    #(#uniform_field_values,)*
                    handle: inner.borrow().handle,
                    bind_group_layout: inner.borrow().bind_group_layout.clone(),
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(VertexAttr)]
pub fn vertex_attr(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let mut attributes = Vec::new();
    let mut sizes = Vec::new();
    let mut shader_location: u32 = 0;

    match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                for field in &fields.named {
                    let field_ident = &field.ty;
                    let size = quote! {
                        (std::mem::size_of::<#field_ident>() as u64)
                    };
                    let offset = quote! {
                        0 #(+ #sizes)*
                    };
                    let format = quote! {
                        < #field_ident as visula_core::VertexAttrFormat >::vertex_attr_format()
                    };
                    attributes.push(quote! {
                        ::visula_core::wgpu::VertexAttribute{
                            format: #format,
                            offset: #offset as u64,
                            shader_location: #shader_location + shader_location_offset,
                        }
                    });
                    sizes.push(size);
                    shader_location += 1;
                }
            }
            Fields::Unnamed(_) | Fields::Unit => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }

    let expanded = quote! {
        impl crate::VertexAttr for #name {
            fn attributes(shader_location_offset: u32) -> Vec<::visula_core::wgpu::VertexAttribute> {
                vec![
                    #( #attributes, )*
                ]
            }
        }
    };

    TokenStream::from(expanded)
}
