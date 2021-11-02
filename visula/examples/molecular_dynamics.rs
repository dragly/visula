use bytemuck::{Pod, Zeroable};
use wgpu::BufferUsages;

use cgmath::InnerSpace;
use itertools_num::linspace;
use naga::{Binding, Expression, FunctionArgument, ResourceBinding, Span, StructMember, TypeInner};
use structopt::StructOpt;

use visula::{
    BindingBuilder, Buffer, BufferBinding, BufferBindingField, Instance, InstanceBinding,
    InstanceField, InstanceHandle, LineDelegate, Lines, NagaType, SphereDelegate, Spheres, Uniform,
    UniformBinding, UniformField, UniformHandle, Vector3, VertexAttrFormat,
    VertexBufferLayoutBuilder,
};
use visula_derive::{delegate, Instance, Uniform};

#[derive(StructOpt)]
struct Cli {
    #[structopt(long)]
    count: Option<usize>,
}

#[derive(Clone, Debug)]
struct Particle {
    position: Vector3,
    velocity: Vector3,
    acceleration: Vector3,
    mass: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct ParticleData {
    position: [f32; 3],
    radius: f32,
}

trait TwoBodyForce {
    fn force(&self, position_a: &Vector3, position_b: &Vector3) -> Vector3;
}

struct LennardJones {
    eps: f32,
    sigma: f32,
}

impl Default for LennardJones {
    fn default() -> Self {
        Self {
            eps: 1.3,
            sigma: 4.0,
        }
    }
}

impl TwoBodyForce for LennardJones {
    fn force(&self, position_a: &Vector3, position_b: &Vector3) -> Vector3 {
        let Self { eps, sigma, .. } = self;
        let r = position_a - position_b;
        let r_l = r.magnitude();

        r / r_l.powi(2)
            * eps.powi(24)
            * (2.0 * sigma.powi(12) / r_l.powi(12) - sigma.powi(6) / r_l.powi(6))
    }
}

fn integrate<F: TwoBodyForce>(
    out_state: &mut Vec<Particle>,
    in_state: &[Particle],
    two_body: F,
    dt: f32,
) {
    for (out_particle, in_particle) in out_state.iter_mut().zip(in_state.iter()) {
        out_particle.velocity += 0.5 * in_particle.acceleration * dt;
        out_particle.position += out_particle.velocity * dt;
    }
    for particle in out_state.iter_mut() {
        particle.acceleration = Vector3::new(0.0, 0.0, 0.0);
    }
    let intermediate_state = out_state.clone();
    for i in 0..in_state.len() {
        for j in 0..in_state.len() {
            if i == j {
                continue;
            }
            out_state[i].acceleration += two_body.force(
                &intermediate_state[i].position,
                &intermediate_state[j].position,
            ) / intermediate_state[i].mass;
        }
    }
    for particle in out_state.iter_mut() {
        particle.velocity += 0.5 * particle.acceleration * dt;
    }
}

fn generate(count: usize) -> Vec<Particle> {
    let mut current_particles: Vec<Particle> = Vec::new();
    let side = 4.4 * count as f32;
    let start = -side / 2.0;
    let end = side / 2.0;
    for x in linspace(start, end, count) {
        for y in linspace(start, end, count) {
            for z in linspace(start, end, count) {
                current_particles.push(Particle {
                    position: Vector3::new(x, y, z),
                    velocity: Vector3::new(0.0, 0.0, 0.0),
                    acceleration: Vector3::new(0.0, 0.0, 0.0),
                    mass: 1.0,
                })
            }
        }
    }
    current_particles
}

#[derive(Debug)]
struct Error {}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Uniform, Zeroable)]
struct Settings {
    radius: f32,
    width: f32,
}

struct Simulation {
    particles: Vec<Particle>,
    spheres: Spheres,
    particle_buffer: Buffer<ParticleData>,
    lines: Lines,
    settings: Settings,
    settings_buffer: Buffer<Settings>,
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn init(application: &mut visula::Application) -> Result<Simulation, Error> {
        let cli = Cli::from_args();
        let count = cli.count.unwrap_or(6);
        let particles = generate(count);

        // TODO split into UniformBuffer and InstanceBuffer to avoid having UNIFORM usage on all
        let particle_buffer = Buffer::<ParticleData>::new(
            application,
            BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::COPY_DST,
            "particle",
        );
        let particle = particle_buffer.instance();
        let settings_data = Settings {
            radius: 1.0,
            width: 0.3,
        };
        let settings_buffer = Buffer::new_with_init(
            application,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[settings_data],
            "settings",
        );
        let settings = settings_buffer.uniform();
        let spheres = Spheres::new(
            application,
            &SphereDelegate {
                position: delegate!(particle.position),
                radius: delegate!(settings.radius),
            },
        )
        .unwrap();

        let lines = Lines::new(
            application,
            &LineDelegate {
                start: delegate!(particle.position),
                end: delegate!(1.4 * particle.position),
                width: delegate!(settings.width),
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
        })
    }

    fn update(&mut self, application: &visula::Application) {
        let previous_particles = self.particles.clone();
        integrate(
            &mut self.particles,
            &previous_particles,
            LennardJones::default(),
            0.01,
        );
        let particle_data: Vec<ParticleData> = self
            .particles
            .iter()
            .map(|particle| ParticleData {
                position: particle.position.into(),
                radius: particle.mass,
            })
            .collect();
        self.particle_buffer.update(application, &particle_data);
    }

    fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>) {
        let bindings: &[&dyn InstanceBinding] = &[&self.particle_buffer, &self.settings_buffer];
        self.spheres.render(render_pass, &bindings);
        self.lines.render(render_pass, &bindings);
    }
}

fn main() {
    visula::run::<Simulation>();
}
