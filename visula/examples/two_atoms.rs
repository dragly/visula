use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use std::time::Instant;
use visula::{
    simulation::RenderData, BindingBuilder, BufferBinding, BufferBindingField, Expression,
    Instance, InstanceBuffer, InstanceBufferInner, InstanceDeviceExt, InstanceField,
    InstanceHandle, LineDelegate, Lines, NagaType, SphereDelegate, Spheres, Uniform,
    UniformBinding, UniformBuffer, UniformBufferInner, UniformField, UniformHandle,
    VertexAttrFormat, VertexBufferLayoutBuilder,
};
use visula_derive::{Instance, Uniform};

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Instance, Pod, Zeroable)]
struct Particle {
    position: glam::Vec3,
    velocity: glam::Vec3,
    acceleration: glam::Vec3,
    mass: f32,
    _padding: [f32; 2],
}

struct LennardJones {
    eps: f32,
    sigma: f32,
}

impl Default for LennardJones {
    fn default() -> Self {
        Self {
            eps: 1.3,
            sigma: 10.0,
        }
    }
}

impl LennardJones {
    fn force(&self, position_a: &Vec3, position_b: &Vec3) -> Vec3 {
        let Self { eps, sigma, .. } = self;
        let r = *position_a - *position_b;
        let r_l = r.length();

        r / r_l.powi(2)
            * eps.powi(24)
            * (2.0 * sigma.powi(12) / r_l.powi(12) - sigma.powi(6) / r_l.powi(6))
    }
}

#[derive(Debug)]
struct Error {}

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Uniform, Zeroable)]
struct Settings {
    radius: f32,
    width: f32,
    distance: f32,
    _padding: f32,
}

struct Simulation {
    particles: Vec<Particle>,
    spheres: Spheres,
    particle_buffer: InstanceBuffer<Particle>,
    lines: Lines,
    settings: Settings,
    settings_buffer: UniformBuffer<Settings>,
    last_update: Instant,
}

fn generate() -> Vec<Particle> {
    [
        Particle {
            position: glam::Vec3::new(-2.0, 0.0, 0.0),
            velocity: glam::Vec3::new(0.0, 0.0, 0.0),
            acceleration: glam::Vec3::new(0.0, 0.0, 0.0),
            mass: 1.0,
            _padding: [0.0, 0.0],
        },
        Particle {
            position: glam::Vec3::new(2.0, 0.0, 0.0),
            velocity: glam::Vec3::new(0.0, 0.0, 0.0),
            acceleration: glam::Vec3::new(0.0, 0.0, 0.0),
            mass: 1.0,
            _padding: [0.0, 0.0],
        },
    ]
    .into()
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn init(application: &mut visula::Application) -> Result<Simulation, Error> {
        let particles = generate();

        let particle_buffer = application.device.create_instance_buffer::<Particle>();
        let particle = particle_buffer.instance();

        let settings_data = Settings {
            radius: 0.5,
            width: 0.4,
            distance: 10.0,
            _padding: 0.0,
        };
        let settings_buffer = UniformBuffer::new_with_init(&application.device, &settings_data);
        let settings = settings_buffer.uniform();
        let pos = particle.position.clone();
        let spheres = Spheres::new(
            &application.rendering_descriptor(),
            &SphereDelegate {
                position: pos,
                radius: 1.0 + settings.radius,
                color: &particle.position / 40.0 + glam::Vec3::new(0.1, 0.3, 0.8),
            },
        )
        .unwrap();

        let lines = Lines::new(
            &application.rendering_descriptor(),
            &LineDelegate {
                start: particle.position.clone(),
                end: &particle.position + particle.acceleration,
                width: settings.width,
                alpha: 1.0.into(),
            },
        )
        .unwrap();

        Ok(Simulation {
            particles,
            spheres,
            particle_buffer,
            lines,
            settings: settings_data,
            settings_buffer,
            last_update: Instant::now(),
        })
    }

    fn update(&mut self, application: &visula::Application) {
        self.particles[0].position.x = -self.settings.distance / 2.0;
        self.particles[1].position.x = self.settings.distance / 2.0;
        let lj = LennardJones::default();
        let current_time = Instant::now();

        for particle in &mut self.particles {
            particle.acceleration = glam::Vec3::new(0.0, 0.0, 0.0);
        }
        for i in 0..self.particles.len() {
            for j in (i + 1)..self.particles.len() {
                let force = lj.force(&self.particles[i].position, &self.particles[j].position);
                self.particles[i].acceleration += force;
                self.particles[j].acceleration -= force;
            }
        }
        self.particle_buffer
            .update(&application.device, &application.queue, &self.particles);
        self.settings_buffer
            .update(&application.queue, &self.settings);
        self.last_update = current_time;
    }

    fn render(&mut self, data: &mut RenderData) {
        self.spheres.render(data);
        self.lines.render(data);
    }

    fn gui(&mut self, context: &egui::Context) {
        egui::Window::new("Settings").show(context, |ui| {
            ui.label("Distance");
            ui.add(egui::Slider::new(&mut self.settings.distance, 8.0..=20.0));
        });
    }
}

fn main() {
    visula::run::<Simulation>();
}
