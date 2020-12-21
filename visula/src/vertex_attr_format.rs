pub trait VertexAttrFormat {
    fn vertex_attr_format() -> wgpu::VertexFormat;
}

impl VertexAttrFormat for f32 {
    fn vertex_attr_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float
    }
}

impl VertexAttrFormat for [f32; 2] {
    fn vertex_attr_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float2
    }
}

impl VertexAttrFormat for [f32; 3] {
    fn vertex_attr_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float3
    }
}

impl VertexAttrFormat for [f32; 4] {
    fn vertex_attr_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float4
    }
}
