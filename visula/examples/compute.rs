use bytemuck::{Pod, Zeroable};
use wgpu::BufferUsages;

use visula::{
    BindingBuilder, Buffer, BufferBinding, BufferBindingField, BufferInner, Compute,
    ComputeDelegate, Expression, Instance, InstanceField, InstanceHandle, LineDelegate, Lines,
    NagaType, SimulationRenderData, VertexAttrFormat, VertexBufferLayoutBuilder,
};
use visula_derive::Instance;

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct ParticleData {
    position: [f32; 3],
    velocity: [f32; 3],
    _padding: [f32; 2],
}

#[derive(Debug)]
struct Error {}

struct Simulation {
    particle_buffer: Buffer<ParticleData>,
    compute: Compute,
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn init(application: &mut visula::Application) -> Result<Simulation, Error> {
        let particle_buffer = Buffer::<ParticleData>::new(
            application,
            BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::COPY_DST,
            "line",
        );
        let particle = particle_buffer.instance();

        let compute = Compute::new(application, &ComputeDelegate {
            implementation: || {
                particle.position = particle.position + particle.velocity;
            }
        }).unwrap();

        let particle_data = vec![ParticleData {
            position: [-10.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            _padding: [0.0; 2],
        }];

        Ok(Simulation {
            particle_buffer,
            compute,
        })
    }

    fn update(&mut self, application: &visula::Application) {
        self.compute.dispatch(&application);
    }
}

fn main() {
    visula::run::<Simulation>();
}
