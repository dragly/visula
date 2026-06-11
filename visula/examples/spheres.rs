use bytemuck::{Pod, Zeroable};
use visula::{
    vec3, InstanceBuffer, RenderData, Renderable, SphereGeometry, SphereMaterial, Spheres,
};
use visula_derive::Instance;

#[repr(C)]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct Particle {
    t: f32,
}

struct Simulation {
    spheres: Spheres,
    _buffer: InstanceBuffer<Particle>,
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Simulation {
        let data: Vec<Particle> = (0..100_000)
            .map(|i| Particle {
                t: i as f32 * 0.001,
            })
            .collect();
        let buffer = InstanceBuffer::new_with_init(&application.device, &data);
        let t = buffer.instance().t;

        let position = 10.0 * vec3(t.cos(), t.sin(), t);
        let spheres = Spheres::new(
            &application.rendering_descriptor(),
            &SphereGeometry {
                position: position.clone(),
                radius: 0.2.into(),
                color: position / 4.0,
            },
            &SphereMaterial::default(),
        )
        .unwrap();

        Simulation {
            spheres,
            _buffer: buffer,
        }
    }
}

impl visula::Simulation for Simulation {
    fn render(&mut self, data: &mut RenderData) {
        self.spheres.render(data);
    }
}

fn main() {
    visula::run(Simulation::new);
}
