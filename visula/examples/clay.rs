use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use bytemuck::{Pod, Zeroable};

use cgmath::InnerSpace;
use itertools_num::linspace;
use structopt::StructOpt;

use visula::{
    BindingBuilder, Buffer, BufferBinding, BufferBindingField, BufferInner, Expression, Instance,
    InstanceField, InstanceHandle, NagaType, RenderData, SphereDelegate, Spheres, Uniform,
    UniformBinding, UniformField, UniformHandle, Vector3, VertexAttrFormat,
    VertexBufferLayoutBuilder,
};
use visula_derive::{Instance, Uniform};

#[derive(StructOpt)]
struct Cli {
    #[structopt(long)]
    count: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
struct Particle {
    position: Vector3,
    velocity: Vector3,
    acceleration: Vector3,
    mass: f32,
    id: usize,
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

#[derive(Clone, Debug)]
enum SphereTree {
    Empty,
    Leaf {
        position: Vector3,
        id: usize,
    },
    Branch {
        position: Vector3,
        radius: f32,
        left: Rc<RefCell<SphereTree>>,
        right: Rc<RefCell<SphereTree>>,
    },
}

fn node_distance(position: &Vector3, node: &SphereTree) -> f32 {
    let node_position = match node {
        SphereTree::Empty => {
            panic!("Encountered empty leaf node");
        }
        SphereTree::Leaf { position, .. } => position,
        SphereTree::Branch { position, .. } => position,
    };

    (position - node_position).magnitude()
}

fn is_near(position: &Vector3, node: &SphereTree) -> bool {
    let threshold = 8.0f32;
    let threshold2 = threshold.powi(2);
    match node {
        SphereTree::Empty => {
            panic!("Encountered empty leaf node");
        }
        SphereTree::Leaf {
            position: node_position,
            ..
        } => (position - node_position).magnitude2() <= threshold2,
        SphereTree::Branch {
            position: node_position,
            radius,
            ..
        } => {
            let threshold_radius2 = (threshold + radius).powi(2);
            (position - node_position).magnitude2() <= threshold_radius2
        }
    }
}

fn integrate<F: TwoBodyForce>(
    out_state: &mut [Particle],
    in_state: &[Particle],
    two_body: F,
    dt: f32,
) {
    let g = crude_profiler::push("integrate");
    g.replace("position");
    for (out_particle, in_particle) in out_state.iter_mut().zip(in_state.iter()) {
        out_particle.velocity += 0.5 * in_particle.acceleration * dt;
        out_particle.position += out_particle.velocity * dt;
    }
    for out_particle in out_state.iter_mut() {
        out_particle.acceleration = Vector3::new(0.0, 0.0, 0.0);
    }
    let intermediate_state = out_state.to_vec();
    let gravity = Vector3::new(0.0, -9.81, 0.0);
    let bounds_min = Vector3::new(-40.0, -10.0, -40.0);
    let bounds_max = Vector3::new(40.0, 100.0, 40.0);

    let sphere_tree = Rc::new(RefCell::new(SphereTree::Empty));

    g.replace("tree building");
    for particle in &intermediate_state {
        let mut queue: VecDeque<Rc<RefCell<SphereTree>>> = VecDeque::new();
        let mut current_node;
        queue.push_front(sphere_tree.clone());
        loop {
            current_node = match queue.pop_back() {
                Some(node) => node,
                None => {
                    break;
                }
            };
            let mut new_node = None;
            match &*current_node.borrow() {
                SphereTree::Empty => {
                    new_node = Some(SphereTree::Leaf {
                        position: particle.position,
                        id: particle.id,
                    });
                }
                SphereTree::Leaf { position, id } => {
                    let center = (particle.position + *position) / 2.0;
                    let radius = (particle.position - *position).magnitude() / 2.0;
                    new_node = Some(SphereTree::Branch {
                        position: center,
                        radius,
                        left: Rc::new(RefCell::new(SphereTree::Leaf {
                            position: particle.position,
                            id: particle.id,
                        })),
                        right: Rc::new(RefCell::new(SphereTree::Leaf {
                            position: *position,
                            id: *id,
                        })),
                    });
                }
                SphereTree::Branch {
                    left,
                    right,
                    radius,
                    position,
                } => {
                    let distance = (particle.position - *position).magnitude();
                    if distance > *radius {
                        new_node = Some(SphereTree::Branch {
                            position: *position,
                            radius: distance,
                            left: Rc::new(RefCell::new(SphereTree::Leaf {
                                position: particle.position,
                                id: particle.id,
                            })),
                            right: Rc::new(RefCell::new(SphereTree::Branch {
                                left: left.clone(),
                                right: right.clone(),
                                radius: *radius,
                                position: *position,
                            })),
                        });
                    }
                }
            }
            if let Some(node) = new_node {
                *current_node.borrow_mut() = node;
                break;
            }
            if let SphereTree::Branch {
                left,
                right,
                radius,
                position,
            } = &mut *current_node.borrow_mut()
            {
                *radius = radius.max((particle.position - *position).magnitude());
                if node_distance(&particle.position, &left.borrow())
                    < node_distance(&particle.position, &right.borrow())
                {
                    queue.push_front(left.clone());
                } else {
                    queue.push_front(right.clone());
                }
            }
        }
    }

    let mut queue: VecDeque<Rc<RefCell<SphereTree>>> = VecDeque::new();
    g.replace("tree iteration");
    for (intermediate_particle, out_particle) in intermediate_state.iter().zip(out_state.iter_mut())
    {
        queue.push_front(sphere_tree.clone());
        let mut current_node;
        loop {
            current_node = match queue.pop_back() {
                Some(node) => node,
                None => {
                    break;
                }
            };
            match &*current_node.borrow() {
                SphereTree::Empty => {
                    panic!("Encountered empty node in `sphere_tree`");
                }
                SphereTree::Leaf { position, id } => {
                    if intermediate_particle.id != *id {
                        out_particle.acceleration += two_body
                            .force(&intermediate_particle.position, position)
                            / intermediate_particle.mass;
                    }
                }
                SphereTree::Branch { left, right, .. } => {
                    for node in [left, right] {
                        if is_near(&intermediate_particle.position, &node.borrow()) {
                            queue.push_front(node.clone());
                        }
                    }
                }
            }
        }
    }
    g.replace("bounds");
    for (intermediate_particle, out_particle) in intermediate_state.iter().zip(out_state.iter_mut())
    {
        out_particle.acceleration += gravity / out_particle.mass;
        let mut bounds_force_min = -(intermediate_particle.position - bounds_min);
        let mut bounds_force_max = -(intermediate_particle.position - bounds_max);
        bounds_force_min.x = bounds_force_min.x.max(0.0);
        bounds_force_min.y = bounds_force_min.y.max(0.0);
        bounds_force_min.z = bounds_force_min.z.max(0.0);
        bounds_force_max.x = bounds_force_max.x.min(0.0);
        bounds_force_max.y = bounds_force_max.y.min(0.0);
        bounds_force_max.z = bounds_force_max.z.min(0.0);
        let bounds_force = 6.0 * (bounds_force_min + bounds_force_max);
        out_particle.acceleration += bounds_force / out_particle.mass;

        g.replace("damping");
        let damping =
            -intermediate_particle.velocity * intermediate_particle.velocity.magnitude2() / 100.0;
        out_particle.acceleration += damping / out_particle.mass;
    }
    g.replace("freezing");
    for particle in out_state.iter_mut() {
        particle.velocity += 0.5 * particle.acceleration * dt;
        if particle.velocity.magnitude2() < 0.1 {
            particle.velocity = Vector3::new(0.0, 0.0, 0.0);
        }
    }
}

fn generate(count: usize) -> Vec<Particle> {
    let mut current_particles: Vec<Particle> = Vec::new();
    let side = 4.4 * count as f32;
    let start = -side / 2.0;
    let end = side / 2.0;
    let mut id = 0;
    for x in linspace(start, end, count) {
        for y in linspace(start, end, count) {
            for z in linspace(start, end, count) {
                current_particles.push(Particle {
                    position: Vector3::new(x, y, z),
                    velocity: Vector3::new(0.0, 0.0, 0.0),
                    acceleration: Vector3::new(0.0, 0.0, 0.0),
                    mass: 1.0,
                    id,
                });
                id += 1;
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
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn init(application: &mut visula::Application) -> Result<Simulation, Error> {
        let cli = Cli::from_args();
        let count = cli.count.unwrap_or(6);
        let particles = generate(count);

        // TODO split into UniformBuffer and InstanceBuffer to avoid having UNIFORM usage on all
        let particle_buffer = Buffer::<ParticleData>::new(&application.device);
        let particle = particle_buffer.instance();
        let settings_data = Settings {
            radius: 2.0,
            width: 0.3,
        };
        let settings_buffer = Buffer::new_with_init(&application.device, &[settings_data]);
        let settings = settings_buffer.uniform();
        let spheres = Spheres::new(
            &application.rendering_descriptor(),
            &SphereDelegate {
                position: particle.position,
                radius: settings.radius,
                color: glam::Vec3::new(0.2, 0.8, 0.6).into(),
            },
        )
        .unwrap();

        Ok(Simulation {
            particles,
            spheres,
            particle_buffer,
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
        self.particle_buffer
            .update(&application.device, &application.queue, &particle_data);
    }

    fn render(&mut self, data: &mut RenderData) {
        self.spheres.render(data);
    }
}

fn main() {
    visula::run::<Simulation>();
}
