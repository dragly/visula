use wgpu::RenderPassDescriptor;

pub struct DefaultRenderPassDescriptor<'a> {
    label: String,
    color_attachments: [Option<wgpu::RenderPassColorAttachment<'a>>; 1],
    depth_texture: &'a wgpu::TextureView,
}

impl<'b> DefaultRenderPassDescriptor<'b> {
    pub fn new<'a>(
        label: &'a str,
        view: &'a wgpu::TextureView,
        depth_texture: &'a wgpu::TextureView,
    ) -> DefaultRenderPassDescriptor<'a> {
        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
            },
        })];
        DefaultRenderPassDescriptor {
            color_attachments,
            label: label.to_string(),
            depth_texture,
        }
    }
    pub fn build(&self) -> RenderPassDescriptor {
        wgpu::RenderPassDescriptor {
            label: Some(&self.label),
            color_attachments: &self.color_attachments,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.depth_texture,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                }),
                stencil_ops: None,
            }),
        }
    }
}
