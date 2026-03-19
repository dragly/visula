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
            let pyclass_attribute: TokenStream2 =
                format!("#[::pyo3::pyclass(name = \"{struct_ident}\", unsendable)]")
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
                        ::visula_core::inject::inject(module, binding_builder,  shader_variable_name, &fields);
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

    let mut field_index: usize = 0;
    let mut instance_struct_fields = Vec::new();
    let mut instance_field_values = Vec::new();
    let mut field_descriptors = Vec::new();

    let instance_struct_name = format_ident!("{}Instance", name);

    match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                for field in &fields.named {
                    let field_name = &field.ident;
                    match field_name {
                        Some(field_name) => {
                            let field_type = &field.ty;
                            instance_struct_fields.push(quote! {
                                pub #field_name: visula_core::Expression
                            });
                            instance_field_values.push(quote! {
                                #field_name: visula_core::Expression::InstanceField(visula_core::InstanceField {
                                    buffer_handle: inner.borrow().handle,
                                    inner: inner.clone(),
                                    field_index: #field_index,
                                    integrate_instance: #instance_struct_name::integrate,
                                })
                            });
                            field_descriptors.push(quote! {
                                visula_core::InstanceFieldDescriptor {
                                    name: stringify!(#field_name).into(),
                                    naga_type: < #field_type as visula_core::NagaType >::naga_type(),
                                    vertex_attr_format: < #field_type as visula_core::VertexAttrFormat >::vertex_attr_format(),
                                    size: std::mem::size_of::<#field_type>() as u64,
                                }
                            });
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
                let fields = vec![
                    #(#field_descriptors,)*
                ];
                visula_core::integrate_instance(
                    &fields,
                    std::mem::size_of::<#name>() as ::visula_core::wgpu::BufferAddress,
                    inner,
                    handle,
                    module,
                    binding_builder,
                );
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

    let mut uniform_struct_fields = Vec::new();
    let mut uniform_field_values = Vec::new();
    let mut field_descriptors = Vec::new();
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
                            uniform_field_values.push(quote! {
                                #field_name: visula_core::Expression::UniformField(visula_core::UniformField {
                                    buffer_handle: inner.borrow().handle,
                                    inner: inner.clone(),
                                    field_index: #field_index,
                                    bind_group_layout: inner.borrow().bind_group_layout.clone(),
                                    integrate_uniform: ::std::rc::Rc::new(::std::cell::RefCell::new(#uniform_struct_name::integrate)),
                                })
                            });
                            field_descriptors.push(quote! {
                                visula_core::UniformFieldDescriptor {
                                    name: stringify!(#field_name).into(),
                                    naga_type: < #field_type as visula_core::NagaType >::naga_type(),
                                    size: std::mem::size_of::<#field_type>() as u32,
                                }
                            });
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

    let name_lower = name.to_string().to_lowercase();

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
                let fields = vec![
                    #(#field_descriptors,)*
                ];
                let params = visula_core::IntegrateUniformParams {
                    struct_name: stringify!(#uniform_struct_name),
                    variable_name: #name_lower,
                    fields: &fields,
                    struct_span: std::mem::size_of::<#uniform_struct_name>() as u32,
                    inner,
                    handle,
                    bind_group_layout,
                };
                visula_core::integrate_uniform(&params, module, binding_builder);
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

    let mut field_descriptors = Vec::new();

    match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                for field in &fields.named {
                    let field_type = &field.ty;
                    field_descriptors.push(quote! {
                        (
                            < #field_type as visula_core::VertexAttrFormat >::vertex_attr_format(),
                            std::mem::size_of::<#field_type>() as u64,
                        )
                    });
                }
            }
            Fields::Unnamed(_) | Fields::Unit => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }

    let expanded = quote! {
        impl crate::VertexAttr for #name {
            fn attributes(shader_location_offset: u32) -> Vec<::visula_core::wgpu::VertexAttribute> {
                let fields = vec![
                    #(#field_descriptors,)*
                ];
                visula_core::compute_vertex_attributes(&fields, shader_location_offset)
            }
        }
    };

    TokenStream::from(expanded)
}
