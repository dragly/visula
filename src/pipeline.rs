pub struct Pipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: usize,
    pub instance_buffer: wgpu::Buffer,
    pub instance_count: usize,
    pub uniform_buffer: wgpu::Buffer,
}
