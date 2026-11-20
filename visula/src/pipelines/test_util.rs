use crate::post_process::transparency::TransparencyPass;
use crate::simulation::TransparencyRenderData;
use crate::{Camera, DirectionalLight, RenderData, RenderingDescriptor};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub struct TestScene {
    pub device: wgpu::Device,
    queue: wgpu::Queue,
    camera: Camera,
    light: DirectionalLight,
}

impl TestScene {
    pub fn new() -> Self {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits {
                max_bind_groups: 8,
                ..Default::default()
            },
            ..Default::default()
        });
        let camera = Camera::new(&device);
        let light = DirectionalLight::new(&device);
        TestScene {
            device,
            queue,
            camera,
            light,
        }
    }

    pub fn rendering_descriptor(&self) -> RenderingDescriptor<'_> {
        RenderingDescriptor {
            device: &self.device,
            format: FORMAT,
            camera: &self.camera,
            light: &self.light,
            sample_count: 1,
        }
    }

    pub fn render(&self, render: impl FnOnce(&mut RenderData)) {
        let color = self.render_target(FORMAT);
        let normal = self.render_target(wgpu::TextureFormat::Rgba16Float);
        let depth = self.render_target(wgpu::TextureFormat::Depth32Float);
        let transparency = TransparencyPass::new(&self.device, FORMAT, 1, 1, 1);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut transparent_encoder = self.device.create_command_encoder(&Default::default());
        let attachment = |view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations::default(),
            })
        };
        let pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[attachment(&color), attachment(&normal)],
                depth_stencil_attachment: Some(depth_attachment(&depth)),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            })
            .forget_lifetime();
        render(&mut RenderData {
            pass,
            camera: &self.camera,
            light: &self.light,
            transparency: TransparencyRenderData::new(
                &mut transparent_encoder,
                &transparency,
                &depth,
            ),
        });
        self.queue
            .submit([encoder.finish(), transparent_encoder.finish()]);
    }

    fn render_target(&self, format: wgpu::TextureFormat) -> wgpu::TextureView {
        self.device
            .create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    }
}

fn depth_attachment(view: &wgpu::TextureView) -> wgpu::RenderPassDepthStencilAttachment<'_> {
    wgpu::RenderPassDepthStencilAttachment {
        view,
        depth_ops: Some(wgpu::Operations::default()),
        stencil_ops: None,
    }
}
