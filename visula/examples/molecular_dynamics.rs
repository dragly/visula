use std::time::{Duration, Instant};

use bytemuck::{Pod, Zeroable};
use wgpu::BufferUsages;

use itertools_num::linspace;
use structopt::StructOpt;

use glam::Vec3;
use visula::{
    simulation::SimulationRenderData, BindingBuilder, Buffer, BufferBinding, BufferBindingField,
    BufferInner, Expression, ExpressionInner, Instance, InstanceField, InstanceHandle,
    LineDelegate, Lines, NagaType, SphereDelegate, Spheres, Uniform, UniformBinding, UniformField,
    UniformHandle, VertexAttrFormat, VertexBufferLayoutBuilder,
};
use visula_derive::{Instance, Uniform};

#[derive(StructOpt)]
struct Cli {
    #[structopt(long)]
    count: Option<usize>,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Instance, Pod, Zeroable)]
struct Particle {
    position: glam::Vec3,
    velocity: glam::Vec3,
    acceleration: glam::Vec3,
    mass: f32,
    _padding: [f32; 2],
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
    fn force(&self, position_a: &Vec3, position_b: &Vec3) -> Vec3;
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
    fn force(&self, position_a: &Vec3, position_b: &Vec3) -> Vec3 {
        let Self { eps, sigma, .. } = self;
        let r = *position_a - *position_b;
        let r_l = r.length();

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
        particle.acceleration = glam::Vec3::new(0.0, 0.0, 0.0);
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
                (intermediate_state[i].position - intermediate_state[j].position).length_squared();
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
            -1000.0 * particle.velocity * (particle.velocity.length() - target_temperature) * dt;

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
                    position: Vec3::new(x, y, z),
                    velocity: Vec3::new(0.0, 0.0, 0.0),
                    acceleration: Vec3::new(0.0, 0.0, 0.0),
                    mass: 1.0,
                    _padding: [0.0; 2],
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
    min: Vec3,
    max: Vec3,
}

struct Simulation {
    particles: Vec<Particle>,
    spheres: Spheres,
    particle_buffer: Buffer<Particle>,
    lines: Lines,
    settings: Settings,
    settings_buffer: Buffer<Settings>,
    bond_buffer: Buffer<BondData>,
    bounding_box: BoundingBox,
    count: usize,
    target_temperature: f32,
    last_update: Instant,
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
        let particle_buffer = Buffer::<Particle>::new(
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
        let pos = particle.position.clone();
        let spheres = Spheres::new(
            application,
            &SphereDelegate {
                position: pos,
                radius: 1.0 + settings.radius,
                color: particle.position / 40.0 + glam::Vec3::new(0.1, 0.3, 0.8),
            },
        )
        .unwrap();

        let lines = Lines::new(
            application,
            &LineDelegate {
                start: bond.position_a,
                end: bond.position_b,
                width: settings.width,
                alpha: bond.strength,
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
                min: Vec3::new(-bound, -bound, -bound),
                max: Vec3::new(bound, bound, bound),
            },
            count,
            target_temperature: 10.0,
            last_update: Instant::now(),
        })
    }

    fn update(&mut self, application: &visula::Application) {
        let mut bond_data = Vec::new();
        let current_time = Instant::now();
        let time_diff = current_time - self.last_update;
        let target_fps = self.settings.speed as f32 * 60.0;
        if time_diff < Duration::from_secs_f32(1.0 / target_fps) {
            return;
        }
        let steps = ((target_fps * time_diff.as_secs_f32()) as i32).min(self.settings.speed);
        for _ in 0..steps {
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

        self.particle_buffer.update(application, &self.particles);
        self.settings_buffer.update(application, &[self.settings]);
        self.last_update = current_time;
    }

    fn render(&mut self, data: &mut SimulationRenderData) {
        self.spheres.render(data);
        self.lines.render(data);
    }

    fn gui(&mut self, context: &egui::Context) {
        egui::Window::new("Settings").show(context, |ui| {
            ui.label("Simulation speed");
            ui.add(egui::Slider::new(&mut self.settings.speed, 1..=20));
            ui.label("Target temperature");
            ui.add(egui::Slider::new(&mut self.target_temperature, 0.0..=20.0));
            ui.label("Radius");
            ui.add(egui::Slider::new(&mut self.settings.radius, 0.1..=2.0));

            if ui.button("Reset").clicked() {
                self.reset();
            }
        });
    }
}

fn main() {
    visula::run::<Simulation>();
}
