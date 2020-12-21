pub trait Pipeline {
    fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>);
}
