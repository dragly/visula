use wgpu::VertexAttributeDescriptor;

pub trait VertexAttr {
    fn attributes(shader_location_offset: u32) -> Vec<VertexAttributeDescriptor>;
}
