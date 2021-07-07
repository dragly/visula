use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct MeshVertexAttributes {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [u8; 4],
}
