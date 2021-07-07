pub trait VertexAttrFormat {
    fn vertex_attr_format() -> wgpu::VertexFormat;
}

impl VertexAttrFormat for f32 {
    fn vertex_attr_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float32
    }
}

impl VertexAttrFormat for [f32; 2] {
    fn vertex_attr_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float32x2
    }
}

impl VertexAttrFormat for [f32; 3] {
    fn vertex_attr_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float32x3
    }
}

impl VertexAttrFormat for [f32; 4] {
    fn vertex_attr_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float32x4
    }
}
