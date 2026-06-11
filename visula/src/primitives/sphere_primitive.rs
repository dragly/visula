use bytemuck::{Pod, Zeroable};
use visula_derive::*;

#[repr(C)]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
pub struct SpherePrimitive {
    pub position: [f32; 3],
    pub radius: f32,
    pub color: [f32; 3],
}
