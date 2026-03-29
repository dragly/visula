use wgpu::Device;

pub struct ComputePass {
    pipeline: wgpu::ComputePipeline,
}

impl ComputePass {
    pub fn new(
        device: &Device,
        shader_source: &str,
        entry_point: &str,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ComputePass shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ComputePass layout"),
            bind_group_layouts,
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ComputePass pipeline"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some(entry_point),
            compilation_options: Default::default(),
            cache: None,
        });

        Self { pipeline }
    }

    pub fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_groups: &[&wgpu::BindGroup],
        workgroups: [u32; 3],
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ComputePass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        for (i, bg) in bind_groups.iter().enumerate() {
            pass.set_bind_group(i as u32, Some(*bg), &[]);
        }
        pass.dispatch_workgroups(workgroups[0], workgroups[1], workgroups[2]);
    }
}
