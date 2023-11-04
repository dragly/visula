use crate::{pipelines::pipeline::Pipeline, DefaultRenderPassDescriptor, RenderData};

pub struct InstancedPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: usize,
    pub instance_buffer: wgpu::Buffer,
    pub instance_count: usize,
}

impl Pipeline for InstancedPipeline {
    fn render(&mut self, data: &mut RenderData) {
        let RenderData {
            encoder,
            view,
            multisampled_framebuffer,
            depth_texture,
            ..
        } = data;
        let default_render_pass = DefaultRenderPassDescriptor::new(
            "instanced",
            view,
            multisampled_framebuffer,
            depth_texture,
        );
        let mut render_pass = encoder.begin_render_pass(&default_render_pass.build());
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw_indexed(0..self.index_count as u32, 0, 0..self.instance_count as u32);
    }
}
