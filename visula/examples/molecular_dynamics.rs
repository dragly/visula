use bytemuck::{Pod, Zeroable};
use wgpu::BufferUsages;

use cgmath::InnerSpace;
use itertools_num::linspace;
use naga::{ResourceBinding, StructMember, TypeInner};
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

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct ParticleData {
    position: [f32; 3],
    radius: f32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct BondData {
    position_a: [f32; 3],
    position_b: [f32; 3],
    _padding: [f32; 2],
}

trait TwoBodyForce {
    fn force(&self, position_a: &Vector3, position_b: &Vector3) -> Vector3;
    fn bond_magnitude2(&self) -> f32;
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

    fn bond_magnitude2(&self) -> f32 {
        1.6 * self.sigma.powi(2)
    }
}

fn integrate<F: TwoBodyForce>(
    out_state: &mut [Particle],
    in_state: &[Particle],
    two_body: F,
    dt: f32,
) -> Vec<BondData> {
    for (out_particle, in_particle) in out_state.iter_mut().zip(in_state.iter()) {
        out_particle.velocity += 0.5 * in_particle.acceleration * dt;
        out_particle.position += out_particle.velocity * dt;
    }
    for particle in out_state.iter_mut() {
        particle.acceleration = Vector3::new(0.0, 0.0, 0.0);
    }
    let intermediate_state = out_state.to_vec();
    let mut bonds = Vec::new();
    for i in 0..in_state.len() {
        for j in 0..in_state.len() {
            if i == j {
                continue;
            }
            let position_i = &intermediate_state[i].position;
            let position_j = &intermediate_state[j].position;
            out_state[i].acceleration +=
                two_body.force(position_i, position_j) / intermediate_state[i].mass;

            if (intermediate_state[i].position - intermediate_state[j].position).magnitude2() < two_body.bond_magnitude2()
            {
                bonds.push(BondData {
                    position_a: position_i.clone().into(),
                    position_b: position_j.clone().into(),
                    _padding: [0.0; 2],
                });
            }
        }
    }
    for particle in out_state.iter_mut() {
        particle.acceleration -= 0.1 * particle.velocity * (particle.velocity.magnitude2() - 1.0) * dt;
        particle.velocity += 0.5 * particle.acceleration * dt;
    }

    bonds
}

fn generate(count: usize) -> Vec<Particle> {
    let mut current_particles: Vec<Particle> = Vec::new();
    let side = 4.0 * count as f32;
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

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Uniform, Zeroable)]
struct Settings {
    radius: f32,
    width: f32,
    speed: i32,
    _padding: f32,
}

struct Simulation {
    particles: Vec<Particle>,
    spheres: Spheres,
    particle_buffer: Buffer<ParticleData>,
    lines: Lines,
    settings: Settings,
    settings_buffer: Buffer<Settings>,
    bond_buffer: Buffer<BondData>,
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

        let bond_buffer = Buffer::<BondData>::new(
            application,
            BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::COPY_DST,
            "bond",
        );
        let bond = bond_buffer.instance();

        let settings_data = Settings {
            radius: 0.5,
            width: 0.1,
            speed: 8,
            _padding: 0.0
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
                start: delegate!(bond.position_a),
                end: delegate!(bond.position_b),
                width: delegate!(settings.width),
            },
        )
        .unwrap();

        Ok(Simulation {
            particles,
            spheres,
            particle_buffer,
            bond_buffer,
            lines,
            settings: settings_data,
            settings_buffer,
        })
    }

    fn update(&mut self, application: &visula::Application) {
        let mut bond_data = Vec::new();
        for _ in 0..self.settings.speed {
            let previous_particles = self.particles.clone();
            bond_data = integrate(
                &mut self.particles,
                &previous_particles,
                LennardJones::default(),
                0.005,
            );
        }
        self.bond_buffer.update(application, &bond_data);

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
        {
            let particle_bindings: &[&dyn InstanceBinding] =
                &[&self.particle_buffer, &self.settings_buffer];
            self.spheres.render(render_pass, particle_bindings);
        }
        if self.bond_buffer.count != 0 {
            let bond_bindings: &[&dyn InstanceBinding] =
                &[&self.bond_buffer, &self.settings_buffer];
            self.lines.render(render_pass, bond_bindings);
        }
    }
}

fn main() {
    visula::run::<Simulation>();
}
