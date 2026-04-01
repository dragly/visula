use wgpu::RenderPassDescriptor;

pub struct DefaultRenderPassDescriptor<'a> {
    label: String,
    color_attachments: [Option<wgpu::RenderPassColorAttachment<'a>>; 2],
    depth_texture: &'a wgpu::TextureView,
}

impl DefaultRenderPassDescriptor<'_> {
    pub fn new<'a>(
        label: &'a str,
        view: &'a wgpu::TextureView,
        multisampled_framebuffer: &'a wgpu::TextureView,
        depth_texture: &'a wgpu::TextureView,
        normal_msaa: &'a wgpu::TextureView,
        normal_resolve: &'a wgpu::TextureView,
    ) -> DefaultRenderPassDescriptor<'a> {
        let color_attachments = [
            Some(wgpu::RenderPassColorAttachment {
                view: multisampled_framebuffer,
                resolve_target: Some(view),
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: normal_msaa,
                resolve_target: Some(normal_resolve),
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
        ];
        DefaultRenderPassDescriptor {
            color_attachments,
            label: label.to_string(),
            depth_texture,
        }
    }
    pub fn build(&self) -> RenderPassDescriptor<'_> {
        wgpu::RenderPassDescriptor {
            label: Some(&self.label),
            color_attachments: &self.color_attachments,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.depth_texture,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        }
    }
}
