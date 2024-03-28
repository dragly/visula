use crate::simulation::SimulationRenderData;
use crate::{Application, DefaultRenderPassDescriptor, Expression};
use crate::{BindingBuilder, BufferBinding};
use bytemuck::{Pod, Zeroable};
use naga::{back::wgsl::WriterFlags, valid::ValidationFlags, Block, Statement};
use std::cell::Ref;
use std::mem::size_of;
use visula_derive::Delegate;
use wgpu::util::DeviceExt;
use wgpu::{BindGroupLayout, BufferUsages};

pub struct Compute {
    compute_pipeline: wgpu::ComputePipeline,
    binding_builder: BindingBuilder,
}

#[derive(Delegate)]
pub struct ComputeDelegate {
    implementation: Box<dyn Fn() -> ()>,
}

impl Compute {
    pub fn new(
        application: &mut Application,
        delegate: &ComputeDelegate,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let Application { device, .. } = application;

        let mut module =
            naga::front::wgsl::parse_str(include_str!("../shaders/compute.wgsl")).unwrap();
        let mut binding_builder = BindingBuilder::new(&module, "main", 1);

        delegate.inject("compute", &mut module, &mut binding_builder);

        log::debug!("Validating generated compute shader");
        let info =
            naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
                .validate(&module)
                .unwrap();
        let output_str =
            naga::back::wgsl::write_string(&module, &info, WriterFlags::all()).unwrap();
        log::debug!("Resulting compute shader code:\n{}", output_str);

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&output_str)),
        });

        let bind_group_layouts: Vec<&BindGroupLayout> = binding_builder
            .uniforms
            .values()
            .map(|binding| binding.bind_group_layout.as_ref())
            .collect();

        let uniforms = {
            let mut uniforms: Vec<&BindGroupLayout> = vec![];
            for layout in &bind_group_layouts {
                uniforms.push(layout);
            }
            uniforms
        };
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compute pipeline layout"),
            bind_group_layouts: &uniforms,
            push_constant_ranges: &[],
        });

        let mut layouts = binding_builder
            .bindings
            .values()
            .map(|binding| binding.layout.build())
            .collect();
        let buffers = {
            let mut buffers = vec![];
            buffers.append(&mut layouts);
            buffers
        };
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });
        Ok(Compute {
            binding_builder,
            compute_pipeline,
        })
    }

    pub fn dispatch(&self, application: &Application) {
        let mut encoder = application
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let bindings: Vec<(&BufferBinding, Ref<wgpu::Buffer>)> = self
            .binding_builder
            .bindings
            .values()
            .map(|v| (v, Ref::map(v.inner.borrow(), |v| &v.buffer)))
            .collect();
        let uniforms: Vec<Ref<wgpu::BindGroup>> = self
            .binding_builder
            .uniforms
            .values()
            .map(|v| Ref::map(v.inner.borrow(), |m| &m.bind_group))
            .collect();
        {
            let mut compute_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            compute_pass.set_pipeline(&self.compute_pipeline);
            //for (binding, buffer) in bindings.iter() {
            //let slot = binding.slot;
            //log::debug!("Setting vertex buffer {}", slot);
            //compute_pass.set(slot, buffer.slice(..));
            //instance_count = instance_count.max(binding.inner.borrow().count);
            //}
            for bind_group in uniforms.iter() {
                log::debug!("Setting bind group {}", 1);
                compute_pass.set_bind_group(1, bind_group, &[]);
            }
            compute_pass.dispatch_workgroups(1, 1, 1);
        }
        application.queue.submit(Some(encoder.finish()));
    }
}
