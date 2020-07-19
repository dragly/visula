use crate::Point3;

use bytemuck;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Sphere {
    pub position: Point3,
    pub radius: f32,
    pub color: Point3,
}

unsafe impl Pod for Sphere {}
unsafe impl Zeroable for Sphere {}

