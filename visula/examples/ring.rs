use std::sync::Arc;
use wgpu::TextureViewDescriptor;

use bytemuck::{Pod, Zeroable};
use visula::{
    initialize_event_loop_and_window, initialize_logger, Application, Expression, InstanceBuffer,
    RenderData, SphereDelegate, Spheres,
};
use visula::{Renderable, SphereFragment};
use visula_derive::Instance;
use wgpu::Color;
use winit::event::Event;
use winit::event::WindowEvent;

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct RingData {
    position: [f32; 3],
    radius_x: f32,
}

#[derive(Debug)]
struct Error {}

struct Simulation {
    spheres: Spheres,
    ring_buffer: InstanceBuffer<RingData>,
    ring_data: Vec<RingData>,
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Result<Simulation, Error> {
        let ring_buffer = InstanceBuffer::<RingData>::new(&application.device);
        let ring = ring_buffer.instance();

        let spheres = Spheres::new(
            &application.rendering_descriptor(),
            &SphereDelegate {
                position: ring.position.clone(),
                radius: ring.radius_x,
            },
            &SphereFragment {
                color: Expression::Vector3 {
                    x: 1.0.into(),
                    y: 1.0.into(),
                    z: 1.0.into(),
                },
            },
        )
        .unwrap();

        let ring_data = vec![RingData {
            position: [0.0, 0.0, 0.0],
            radius_x: 10.0,
        }];

        Ok(Simulation {
            spheres,
            ring_buffer,
            ring_data,
        })
    }
}
impl visula::Simulation for Simulation {
    type Error = Error;
    fn update(&mut self, application: &mut visula::Application) {
        self.ring_buffer
            .update(&application.device, &application.queue, &self.ring_data);
    }

    fn render(&mut self, data: &mut RenderData) {
        self.spheres.render(data);
    }
}

fn main() {
    visula::run(|app| Simulation::new(app).expect("Initializing simulation failed"));
}
