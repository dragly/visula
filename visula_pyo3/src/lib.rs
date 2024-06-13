#[cfg(not(target_arch = "wasm32"))]
mod lib_impl;

#[cfg(not(target_arch = "wasm32"))]
pub use lib_impl::*;
