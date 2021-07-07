use crate::VertexAttrFormat;
use bytemuck::{Pod, Zeroable};
use vertex_attr_derive::*;

#[repr(C)]
#[derive(Clone, Copy, VertexAttr)]
pub struct Sphere {
    pub position: [f32; 3],
    pub radius: f32,
    pub color: [f32; 3],
}

unsafe impl Pod for Sphere {}
unsafe impl Zeroable for Sphere {}
