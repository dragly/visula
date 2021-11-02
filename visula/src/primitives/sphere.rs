use crate::{NagaType, VertexAttrFormat};
use bytemuck::{Pod, Zeroable};
use visula_derive::*;

#[repr(C)]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
pub struct Sphere {
    pub position: [f32; 3],
    pub radius: f32,
    pub color: [f32; 3],
    pub padding: f32,
}
