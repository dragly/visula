use proc_macro::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, BinOp, Data, DeriveInput, Expr, ExprBinary, ExprField, Field, Fields,
    FieldsNamed, ItemStruct,
};

type TokenStream2 = proc_macro2::TokenStream;

fn visula_crate_name() -> TokenStream2 {
    let found_crate = crate_name("visula").expect("visula is not present in `Cargo.toml`");

    match found_crate {
        FoundCrate::Itself => quote!(crate),
        FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote!( #ident )
        }
    }
}

fn parse(input: &Expr, setup: &mut Vec<proc_macro2::TokenStream>) -> proc_macro2::TokenStream {
    let current = setup.len();
    let expression_current = format_ident! {"expression_{}", current};
    let constant_current = format_ident! {"constant_{}", current};
    match input {
        Expr::Lit(lit) => {
            setup.push(quote! {
                let #constant_current = module.constants.append(
                    naga::Constant {
                        name: None,
                        specialization: None,
                        inner: naga::ConstantInner::Scalar {
                            width: 4,
                            value: naga::ScalarValue::Float(#lit),
                        },
                    },
                    ::naga::Span::Unknown,
                );
                let #expression_current = module.entry_points[entry_point_index]
                    .function
                    .expressions
                    .append(::naga::Expression::Constant(#constant_current), ::naga::Span::Unknown);
            });
            quote! { #expression_current }
        }
        Expr::Field(ExprField { base, member, .. }) => {
            setup.push(quote! {
                if !binding_builder.bindings.contains_key(&#base.handle) {
                    #base.integrate(module, binding_builder);
                }
                let #expression_current = #base.#member.integrate(module, binding_builder);
            });
            quote! { #expression_current }
        }
        Expr::Binary(ExprBinary {
            left, right, op, ..
        }) => {
            let parsed_left = parse(left, setup);
            let parsed_right = parse(right, setup);
            let operator = match op {
                BinOp::Add(_) => quote! { naga::BinaryOperator::Add },
                BinOp::Mul(_) => quote! { naga::BinaryOperator::Multiply },
                BinOp::Div(_) => quote! { naga::BinaryOperator::Divide },
                BinOp::Sub(_) => quote! { naga::BinaryOperator::Subtract },
                other => unimplemented!("Not implemented: {:#?}", other),
            };
            setup.push(quote! {
                let #expression_current = module.entry_points[entry_point_index].function.expressions.append(
                    ::naga::Expression::Binary {
                        op: #operator,
                        left: #parsed_left,
                        right: #parsed_right,
                    },
                    naga::Span::Unknown,
                );
            });
            quote! { #expression_current }
        }
        other => unimplemented!("Support for {:#?} is not yet implemented", other),
    }
}

#[proc_macro]
pub fn delegate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Expr);
    let mut setup = vec![];
    let output = parse(&input, &mut setup);
    let result = quote! {
        &|module: &mut naga::Module, binding_builder: &mut BindingBuilder| {
            let entry_point_index = binding_builder.entry_point_index;
            #(#setup)*
            #output
        }
    };
    TokenStream::from(result)
}

#[proc_macro]
pub fn define_delegate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::Item);
    let result = match input {
        syn::Item::Struct(ItemStruct {
            vis, ident, fields, ..
        }) => {
            let mut field_delegates: Vec<TokenStream2> = Vec::new();
            let mut field_modifications: Vec<TokenStream2> = Vec::new();
            match fields {
                Fields::Named(FieldsNamed { named, .. }) => {
                    for (index, field) in named.iter().enumerate() {
                        let Field { vis, ident, .. } = field;
                        field_delegates.push(quote! {
                            #vis #ident: &'a dyn Fn(&mut naga::Module, &mut BindingBuilder) -> Handle<naga::Expression>,
                        });
                        field_modifications.push(quote! {
                            {
                                let result_expression = (self.#ident)(module, binding_builder);
                                let access_index = module.entry_points[entry_point_index]
                                    .function
                                    .expressions
                                    .append(
                                        ::naga::Expression::AccessIndex {
                                            index: #index as u32,
                                            base: variable_expression,
                                        },
                                        ::naga::Span::Unknown,
                                    );
                                Statement::Store {
                                    pointer: access_index,
                                    value: result_expression,
                                }
                            },
                        });
                    }
                }
                _ => unimplemented!(),
            };
            quote! {
                #vis struct #ident<'a> {
                    #(#field_delegates)*
                }

                impl<'a> #ident<'a> {
                    fn inject(&self, shader_variable_name: &str, module: &mut naga::Module, binding_builder: &mut BindingBuilder) {
                        let entry_point_index = binding_builder.entry_point_index;
                        let variable = module.entry_points[entry_point_index]
                            .function
                            .local_variables
                            .fetch_if(|variable| variable.name == Some(shader_variable_name.into()))
                            .unwrap();
                        let variable_expression = module.entry_points[entry_point_index]
                            .function
                            .expressions
                            .fetch_if(|expression| match expression {
                                ::naga::Expression::LocalVariable(v) => v == &variable,
                                _ => false,
                            })
                            .unwrap();
                        let mut new_body = Block::from_vec(vec![
                            #(#field_modifications)*
                        ]);
                        for (statement, span) in module.entry_points[entry_point_index]
                            .function
                            .body
                            .span_iter_mut()
                        {
                            new_body.push(
                                statement.clone(),
                                match span {
                                    Some(s) => s.clone(),
                                    None => ::naga::Span::Unknown,
                                },
                            );
                        }
                        module.entry_points[entry_point_index].function.body = new_body;
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
    let crate_name = visula_crate_name();
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
                                < #field_type >::vertex_attr_format()
                            };
                            instance_struct_fields.push(quote! {
                                //#field_name: Handle<naga::Expression>
                                pub #field_name: #crate_name::InstanceField
                            });
                            let naga_type = quote! {
                                < #field_type >::naga_type()
                            };
                            module_fields.push(quote! {
                                {
                                    let field_type = module.types.append(#naga_type, naga::Span::Unknown);
                                    module.entry_points[entry_point_index]
                                        .function
                                        .arguments
                                        .push(::naga::FunctionArgument {
                                            name: Some(stringify!(#field_name).into()),
                                            ty: field_type,
                                            binding: Some(::naga::Binding::Location {
                                                location: previous_shader_location_offset + #shader_location,
                                                interpolation: None,
                                                sampling: None,
                                            }),
                                        });
                                }
                            });
                            instance_field_values.push(quote! {
                                #field_name: #crate_name::InstanceField {
                                    buffer_handle: handle,
                                    field_index: #field_index,
                                }
                            });
                            attributes.push(quote! {
                                wgpu::VertexAttribute{
                                    format: #format,
                                    offset: #offset as u64,
                                    shader_location: previous_shader_location_offset + #shader_location,
                                }
                            });
                            binding_fields.push(quote! {
                                #crate_name::BufferBindingField {
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

    let instance_struct_name = format_ident!("{}Instance", name);

    let expanded = quote! {
        pub struct #instance_struct_name {
            #(#instance_struct_fields,)*
            pub handle: u64,
        }

        impl #crate_name::InstanceHandle for #instance_struct_name {
            fn integrate(
                &self,
                module: &mut naga::Module,
                binding_builder: &mut #crate_name::BindingBuilder,
            )
            {
                let entry_point_index = binding_builder.entry_point_index;
                let previous_shader_location_offset = binding_builder.shader_location_offset;
                let slot = binding_builder.current_slot;

                #(#module_fields)*

                binding_builder.bindings.insert(self.handle, #crate_name::BufferBinding {
                    layout: #crate_name::VertexBufferLayoutBuilder {
                        array_stride: std::mem::size_of::<#name>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: vec![
                            #(#attributes),*
                        ],
                    },
                    slot: binding_builder.current_slot,
                    fields: vec![
                        #(#binding_fields,)*
                    ]
                });

                binding_builder.shader_location_offset += #shader_location;
                binding_builder.current_slot += 1;
            }

        }

        impl #crate_name::Instance for #name {
            type Type = #instance_struct_name;
            fn instance(handle: u64) -> Self::Type {
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

    match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                for field in &fields.named {
                    let field_name = &field.ident;
                    match field_name {
                        Some(field_name) => {
                            let field_type = &field.ty;
                            uniform_struct_fields.push(quote! {
                                #field_name: UniformField
                            });
                            let size = quote! {
                                (std::mem::size_of::<#field_type>() as u32)
                            };
                            let naga_type = quote! {
                                < #field_type >::naga_type()
                            };
                            let field_type_declaration = format_ident!("{}_type", field_name);
                            uniform_field_types_init.push(quote! {
                                let #field_type_declaration = module.types.append(#naga_type, ::naga::Span::Unknown);
                            });
                            let offset = quote! {
                                0 #(+ #sizes)*
                            };
                            uniform_fields.push(quote! {
                                StructMember {
                                    name: Some(stringify!(#field_name).into()),
                                    ty: #field_type_declaration,
                                    binding: None,
                                    offset: #offset,
                                }
                            });
                            uniform_field_values.push(quote! {
                                #field_name: UniformField {
                                    buffer_handle: handle,
                                    field_index: #field_index,
                                }
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

    let uniform_struct_name = format_ident!("{}Uniform", name);

    let expanded = quote! {
        struct #uniform_struct_name {
            #(#uniform_struct_fields,)*
            handle: u64,
            bind_group_layout: std::rc::Rc<wgpu::BindGroupLayout>,
        }

        impl UniformHandle for #uniform_struct_name {
            fn integrate(
                &self,
                module: &mut naga::Module,
                binding_builder: &mut visula::BindingBuilder,
            )
            {
                let entry_point_index = binding_builder.entry_point_index;
                let previous_shader_location_offset = binding_builder.shader_location_offset;
                let slot = binding_builder.current_slot;
                let bind_group = binding_builder.current_bind_group;

                #(#uniform_field_types_init)*

                let uniform_type = module.types.append(
                    naga::Type {
                        name: Some("Settings".into()),
                        inner: TypeInner::Struct {
                            top_level: true,
                            members: vec![
                                #(#uniform_fields),*
                            ],
                            span: 8,
                        },
                    },
                    ::naga::Span::Unknown,
                );
                let uniform_variable = module.global_variables.append(
                    naga::GlobalVariable {
                        name: Some(stringify!(#name).to_lowercase().into()),
                        binding: Some(ResourceBinding {
                            group: bind_group,
                            binding: 0,
                        }),
                        class: naga::StorageClass::Uniform,
                        ty: uniform_type,
                        init: None,
                    },
                    ::naga::Span::Unknown,
                );
                let settings_expression = module.entry_points[entry_point_index]
                    .function
                    .expressions
                    .append(::naga::Expression::GlobalVariable(uniform_variable), ::naga::Span::Unknown);

                binding_builder.uniforms.insert(self.handle, UniformBinding {
                    expression: settings_expression,
                    bind_group_layout: self.bind_group_layout.clone(),
                });
                binding_builder.current_bind_group += 1;
            }

        }

        impl Uniform for #name {
            type Type = #uniform_struct_name;
            fn uniform(handle: u64, bind_group_layout: std::rc::Rc<wgpu::BindGroupLayout>) -> Self::Type {
                Self::Type {
                    #(#uniform_field_values,)*
                    handle,
                    bind_group_layout,
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

    //let mut fields = Vec::new();

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
                        < #field_ident >::vertex_attr_format()
                    };
                    attributes.push(quote! {
                        wgpu::VertexAttribute{
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
            fn attributes(shader_location_offset: u32) -> Vec<wgpu::VertexAttribute> {
                vec![
                    #( #attributes, )*
                ]
            }
        }
    };

    TokenStream::from(expanded)
}