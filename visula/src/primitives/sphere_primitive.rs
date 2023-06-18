use bytemuck::{Pod, Zeroable};
use visula_derive::*;

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
pub struct SpherePrimitive {
    pub position: [f32; 3],
    pub radius: f32,
    pub color: [f32; 3],
    pub padding: f32,
}
