pub mod binding_builder;
pub mod color;
pub mod colormap;
pub mod inject;
pub mod input;
pub mod instance_binding;
pub mod instance_buffer;
pub mod integrate;
pub mod naga_type;
pub mod texture_binding;
pub mod texture_buffer;
pub mod uniform_binding;
pub mod uniform_buffer;
pub mod value;
pub mod vertex_attr;
pub mod vertex_attr_format;

pub mod delegate;
pub mod error;

#[cfg(test)]
extern crate self as visula_core;
#[cfg(test)]
pub mod test_util;

pub use binding_builder::*;
pub use color::*;
pub use colormap::*;
pub use delegate::*;
pub use error::*;
pub use input::*;
pub use instance_binding::*;
pub use instance_buffer::*;
pub use integrate::*;
pub use naga_type::*;
pub use texture_binding::*;
pub use texture_buffer::*;
pub use uniform_binding::*;
pub use uniform_buffer::*;
pub use value::*;
pub use vertex_attr::VertexAttr;
pub use vertex_attr_format::VertexAttrFormat;

pub use glam;
pub use naga;
#[cfg(all(feature = "python", not(target_arch = "wasm32")))]
pub use pyo3;
pub use uuid;
pub use wgpu;

#[cfg(all(feature = "python", not(target_arch = "wasm32")))]
#[doc(hidden)]
#[macro_export]
macro_rules! __if_python {
    ($($tokens:tt)*) => { $($tokens)* };
}

#[cfg(not(all(feature = "python", not(target_arch = "wasm32"))))]
#[doc(hidden)]
#[macro_export]
macro_rules! __if_python {
    ($($tokens:tt)*) => {};
}

pub const LIGHTING_WGSL: &str = include_str!("shaders/lighting.wgsl");
