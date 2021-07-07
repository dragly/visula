use crate::{Matrix4, Vector3};

use bytemuck::{Pod, Zeroable};
use std::mem::size_of;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CameraUniforms {
    pub view_matrix: Matrix4,
    pub model_view_projection_matrix: Matrix4,
    pub center: Vector3,
    pub dummy0: f32,
    pub view_vector: Vector3,
    pub dummy1: f32,
    pub position: Vector3,
    pub dummy2: f32,
    pub up: Vector3,
    pub dummy3: f32,
}

unsafe impl Pod for CameraUniforms {}
unsafe impl Zeroable for CameraUniforms {}

impl AsRef<[f32; size_of::<CameraUniforms>() / size_of::<f32>()]> for CameraUniforms {
    #[inline]
    fn as_ref(&self) -> &[f32; size_of::<CameraUniforms>() / size_of::<f32>()] {
        unsafe {
            &*(self as *const CameraUniforms
                as *const [f32; size_of::<CameraUniforms>() / size_of::<f32>()])
        }
    }
}
