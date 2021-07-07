use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(VertexAttr)]
pub fn my_macro(input: TokenStream) -> TokenStream {
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
