use bytemuck::{Pod, Zeroable};
use wgpu::BufferUsages;

use cgmath::{InnerSpace, Point3};
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
    strength: f32,
    _padding: f32,
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
    target_temperature: f32,
    bounding_box: &BoundingBox,
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

            let distance =
                (intermediate_state[i].position - intermediate_state[j].position).magnitude2();
            let strength = (two_body.bond_magnitude2() - distance) / two_body.bond_magnitude2();

            if strength > 0.0 {
                bonds.push(BondData {
                    position_a: (*position_i).into(),
                    position_b: (*position_j).into(),
                    strength,
                    _padding: 0.0,
                });
            }
        }
    }
    for particle in out_state.iter_mut() {
        let wall_attractiveness = 2.0;

        let mut flipped_xp = particle.position;
        flipped_xp.x = bounding_box.max.x + (bounding_box.max.x - particle.position.x);
        particle.acceleration +=
            wall_attractiveness * two_body.force(&particle.position, &flipped_xp);
        let mut flipped_xm = particle.position;
        flipped_xm.x = bounding_box.min.x + (bounding_box.min.x - particle.position.x);
        particle.acceleration +=
            wall_attractiveness * two_body.force(&particle.position, &flipped_xm);

        let mut flipped_yp = particle.position;
        flipped_yp.y = bounding_box.max.y + (bounding_box.max.y - particle.position.y);
        particle.acceleration += two_body.force(&particle.position, &flipped_yp);
        let mut flipped_ym = particle.position;
        flipped_ym.y = bounding_box.min.y + (bounding_box.min.y - particle.position.y);
        particle.acceleration +=
            wall_attractiveness * two_body.force(&particle.position, &flipped_ym);

        let mut flipped_zp = particle.position;
        flipped_zp.z = bounding_box.max.z + (bounding_box.max.z - particle.position.z);
        particle.acceleration +=
            wall_attractiveness * two_body.force(&particle.position, &flipped_zp);
        let mut flipped_zm = particle.position;
        flipped_zm.z = bounding_box.min.z + (bounding_box.min.z - particle.position.z);
        particle.acceleration +=
            wall_attractiveness * two_body.force(&particle.position, &flipped_zm);

        particle.acceleration +=
            -1000.0 * particle.velocity * (particle.velocity.magnitude() - target_temperature) * dt;

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

struct BoundingBox {
    min: Point3<f32>,
    max: Point3<f32>,
}

struct Simulation {
    particles: Vec<Particle>,
    spheres: Spheres,
    particle_buffer: Buffer<ParticleData>,
    lines: Lines,
    settings: Settings,
    settings_buffer: Buffer<Settings>,
    bond_buffer: Buffer<BondData>,
    bounding_box: BoundingBox,
    count: usize,
    target_temperature: f32,
}

impl Simulation {
    fn reset(&mut self) {
        self.particles = generate(self.count);
    }
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn init(application: &mut visula::Application) -> Result<Simulation, Error> {
        let cli = Cli::from_args();
        let count = cli.count.unwrap_or(8);
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
            width: 0.2,
            speed: 4,
            _padding: 0.0,
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
                color: delegate!(particle.position / 40.0 + vec3::<f32>(0.5, 0.5, 0.5)),
            },
        )
        .unwrap();

        let lines = Lines::new(
            application,
            &LineDelegate {
                start: delegate!(bond.position_a),
                end: delegate!(bond.position_b),
                width: delegate!(settings.width),
                alpha: delegate!(bond.strength),
            },
        )
        .unwrap();

        let bound = count as f32 * 3.0;
        Ok(Simulation {
            particles,
            spheres,
            particle_buffer,
            bond_buffer,
            lines,
            settings: settings_data,
            settings_buffer,
            bounding_box: BoundingBox {
                min: Point3::new(-bound, -bound, -bound),
                max: Point3::new(bound, bound, bound),
            },
            count,
            target_temperature: 10.0,
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
                self.target_temperature,
                &self.bounding_box,
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

    fn gui(&mut self, context: &egui::Context) {
        egui::Window::new("Settings").show(context, |ui| {
            ui.label("Simulation speed");
            ui.add(egui::Slider::new(&mut self.settings.speed, 1..=20));
            ui.label("Target temperature");
            ui.add(egui::Slider::new(&mut self.target_temperature, 0.0..=20.0));

            if ui.button("Reset").clicked() {
                self.reset();
            }
        });
    }
}

fn main() {
    visula::run::<Simulation>();
}
