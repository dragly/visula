use wgpu::Device;

const BLIT_SHADER: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4f {
    // Fullscreen triangle: 3 vertices cover the entire screen
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi & 2u) * 2 - 1);
    return vec4f(x, y, 0.0, 1.0);
}

@group(0) @binding(0) var tex_sampler: sampler;
@group(0) @binding(1) var tex: texture_2d<f32>;

@fragment
fn fs_main(@builtin(position) pos: vec4f) -> @location(0) vec4f {
    let dims = vec2f(textureDimensions(tex));
    let uv = pos.xy / dims;
    return textureSample(tex, tex_sampler, uv);
}
"#;

pub struct BlitPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl BlitPipeline {
    pub fn new(
        device: &Device,
        format: wgpu::TextureFormat,
        sample_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("BlitPipeline shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("BlitPipeline layout"),
            bind_group_layouts: &[sample_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("BlitPipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        sample_bind_group: &wgpu::BindGroup,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("BlitPipeline pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, Some(sample_bind_group), &[]);
        pass.draw(0..3, 0..1);
    }
}
