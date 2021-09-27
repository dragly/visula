use cgmath::InnerSpace;
use itertools_num::linspace;
use num::clamp;
use structopt::StructOpt;
use wgpu::util::DeviceExt;

use visula::{InstancedPipeline, Pipeline, Sphere, Vector3};

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
    _cells: &[Vec<usize>],
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

struct Simulation {
    particles: Vec<Particle>,
    points: InstancedPipeline,
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn init(application: &mut visula::Application) -> Result<Simulation, Error> {
        let cli = Cli::from_args();
        let count = cli.count.unwrap_or(6);
        let points = visula::create_spheres_pipeline(application).unwrap();
        let particles = generate(count);
        Ok(Simulation { particles, points })
    }

    fn update(&mut self, application: &visula::Application) {
        let previous_particles = self.particles.clone();
        let minimum = -2.0;
        let maximum = 2.0;
        let side_length = maximum - minimum;
        let cell_count = 10;
        let mut cells = vec![Vec::new(); 1000];
        for (particle_index, particle) in previous_particles.iter().enumerate() {
            let i: usize = clamp(
                (cell_count as f32 * (particle.position.x - minimum) / side_length) as usize,
                0,
                cell_count - 1,
            );
            let j: usize = clamp(
                (cell_count as f32 * (particle.position.y - minimum) / side_length) as usize,
                0,
                cell_count - 1,
            );
            let k: usize = clamp(
                (cell_count as f32 * (particle.position.z - minimum) / side_length) as usize,
                0,
                cell_count - 1,
            );

            let cell_index = i * cell_count * cell_count + j * cell_count + k;
            cells[cell_index].push(particle_index);
        }
        integrate(
            &mut self.particles,
            &previous_particles,
            &cells,
            LennardJones::default(),
            0.01,
        );
        let points_data: Vec<Sphere> = self
            .particles
            .iter()
            .map(|particle| Sphere {
                position: [
                    particle.position.x,
                    particle.position.y,
                    particle.position.z,
                ],
                color: [1.0, 0.8, 0.2],
                radius: 1.0,
            })
            .collect();
        let instance_buffer =
            application
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Instance buffer"),
                    contents: bytemuck::cast_slice(&points_data),
                    usage: wgpu::BufferUsages::VERTEX,
                });
        self.points.instance_buffer = instance_buffer;
        self.points.instance_count = points_data.len();
        application.window.request_redraw();
    }

    fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.points.render(render_pass);
    }
}

fn main() {
    visula::run::<Simulation>();
}
