use wgpu::VertexAttribute;

pub trait VertexAttr {
    fn attributes(shader_location_offset: u32) -> Vec<VertexAttribute>;
}
