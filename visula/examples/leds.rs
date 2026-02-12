use bytemuck::{Pod, Zeroable};
use clap::Parser;
use dxf::entities::EntityType;
use dxf::Drawing;
use slotmap::{DefaultKey, SlotMap};
use std::path::PathBuf;
use visula::Renderable;

use glam::{Vec2, Vec3, Vec4};
use visula::{
    CustomEvent, Expression, InstanceBuffer, LineDelegate, Lines, RenderData, SphereDelegate,
    Spheres, UniformBuffer,
};
use visula_derive::{Instance, Uniform};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, WindowEvent},
};

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Instance, Pod, Zeroable)]
struct Compartment {
    position: Vec3,
    velocity: Vec3,
    acceleration: Vec3,
    influence: f32,
    _padding: [f32; 2],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Instance, Pod, Zeroable)]
struct Particle {
    position: glam::Vec3,
    voltage: f32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct BondData {
    position_a: Vec3,
    position_b: Vec3,
    strength: f32,
    _padding: f32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct Wall {
    start: Vec3,
    end: Vec3,
    _padding: [f32; 2],
}

fn lennard_jones(position_a: Vec3, position_b: Vec3, eps: f32, sigma: f32) -> Vec3 {
    let r = position_a - position_b;
    let r_l = r.length();

    r / r_l.powi(2)
        * eps.powi(24)
        * (2.0 * sigma.powi(12) / r_l.powi(12) - sigma.powi(6) / r_l.powi(6))
}

#[derive(Debug)]
struct Error {}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Uniform, Zeroable)]
struct Settings {
    radius: f32,
    width: f32,
    speed: i32,
    //_padding: f32,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(value_name = "DXF_PATH")]
    dxf_path: PathBuf,
}

fn load_walls_from_dxf(path: &PathBuf) -> Vec<Wall> {
    let drawing = match Drawing::load_file(path) {
        Ok(drawing) => drawing,
        Err(err) => {
            eprintln!("Failed to read DXF file {}: {}", path.display(), err);
            return Vec::new();
        }
    };

    let mut walls = Vec::new();

    for entity in drawing.entities() {
        match &entity.specific {
            EntityType::Line(line) => {
                walls.push(Wall {
                    start: Vec3::new(line.p1.x as f32 * 10.0, 0.0, line.p1.y as f32 * 10.0),
                    end: Vec3::new(line.p2.x as f32 * 10.0, 0.0, line.p2.y as f32 * 10.0),
                    _padding: [0.0; 2],
                });
            }
            _ => {}
        }
    }

    walls
}

fn closest_point_on_segment_2d(point: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let denom = ab.length_squared();
    if denom == 0.0 {
        return a;
    }
    let t = (point - a).dot(ab) / denom;
    let t = t.clamp(0.0, 1.0);
    a + ab * t
}

struct Mouse {
    position: Option<PhysicalPosition<f64>>,
}

struct Simulation {
    particles: Vec<Particle>,
    spheres: Spheres,
    particle_buffer: InstanceBuffer<Particle>,
    lines: Lines,
    settings: Settings,
    settings_buffer: UniformBuffer<Settings>,
    lines_buffer: InstanceBuffer<BondData>,
    walls: Vec<Wall>,
    wall_lines: Lines,
    wall_buffer: InstanceBuffer<Wall>,
    compartments: SlotMap<DefaultKey, Compartment>,
    mouse: Mouse,
}

impl Simulation {
    fn new(application: &mut visula::Application, walls: Vec<Wall>) -> Result<Simulation, Error> {
        let particle_buffer = InstanceBuffer::<Particle>::new(&application.device);
        let particle = particle_buffer.instance();

        let lines_buffer = InstanceBuffer::<BondData>::new(&application.device);
        let bond = lines_buffer.instance();

        let wall_buffer = InstanceBuffer::<Wall>::new(&application.device);
        let wall = wall_buffer.instance();

        let settings_data = Settings {
            radius: 10.0,
            width: 4.0,
            speed: 4,
            //_padding: 0.0,
        };
        let settings_buffer = UniformBuffer::new_with_init(&application.device, &settings_data);
        let settings = settings_buffer.uniform();
        let pos = &particle.position;
        let spheres = Spheres::new(
            &application.rendering_descriptor(),
            &SphereDelegate {
                position: pos.clone(),
                radius: settings.radius,
                color: Expression::Vector3 {
                    x: (0.1 + (particle.voltage.clone() + 10.0) / 120.0).into(),
                    y: (0.2 + (particle.voltage.clone() + 10.0) / 120.0).into(),
                    z: (0.3 + (particle.voltage.clone() + 10.0) / 120.0).into(),
                },
            },
        )
        .unwrap();

        let lines = Lines::new(
            &application.rendering_descriptor(),
            &LineDelegate {
                start: bond.position_a,
                end: bond.position_b,
                width: settings.width.clone(),
                color: Expression::Vector3 {
                    x: bond.strength.clone().into(),
                    y: 0.8.into(),
                    z: 1.0.into(),
                },
            },
        )
        .unwrap();

        let wall_lines = Lines::new(
            &application.rendering_descriptor(),
            &LineDelegate {
                start: wall.start,
                end: wall.end,
                width: settings.width.clone(),
                color: Expression::Vector3 {
                    x: 0.9.into(),
                    y: 0.2.into(),
                    z: 0.2.into(),
                },
            },
        )
        .unwrap();

        wall_buffer.update(&application.device, &application.queue, &walls);

        let mut compartments = SlotMap::<DefaultKey, Compartment>::new();
        compartments.insert(Compartment {
            position: Vec3::new(0.0, 0.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            acceleration: Vec3::new(0.0, 0.0, 0.0),
            influence: 0.0,
            _padding: Default::default(),
        });
        compartments.insert(Compartment {
            position: Vec3::new(2.0, 0.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            acceleration: Vec3::new(0.0, 0.0, 0.0),
            influence: 0.0,
            _padding: Default::default(),
        });
        compartments.insert(Compartment {
            position: Vec3::new(2.0, 0.0, 2.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            acceleration: Vec3::new(0.0, 0.0, 0.0),
            influence: 0.0,
            _padding: Default::default(),
        });

        Ok(Simulation {
            particles: vec![],
            spheres,
            particle_buffer,
            lines_buffer,
            lines,
            settings: settings_data,
            settings_buffer,
            walls,
            wall_lines,
            wall_buffer,
            compartments,
            mouse: Mouse { position: None },
        })
    }
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn update(&mut self, application: &mut visula::Application) {
        let compartments = &mut self.compartments;
        let dt = 0.01;
        let node_radius = 15.0;
        let connection_distance = 5.0 * node_radius;
        let sigma = 3.0 * node_radius;
        let eps = 1.4;
        let max_velocity = node_radius * 2.0 / 3.0;
        let min_velocity = node_radius * 0.05;
        let wall_padding = node_radius * 0.8;
        let walls = &self.walls;
        let mut bonds = vec![];
        for _ in 0..self.settings.speed {
            for (_key, compartment) in compartments.iter_mut() {
                compartment.velocity += compartment.acceleration * dt;
                if compartment.velocity.length() > max_velocity {
                    compartment.velocity =
                        compartment.velocity / compartment.velocity.length() * max_velocity;
                }
                if compartment.velocity.length() < min_velocity {
                    compartment.velocity *= 0.0;
                }
                compartment.position += compartment.velocity * dt;

                for wall in walls {
                    let p = Vec2::new(compartment.position.x, compartment.position.z);
                    let a = Vec2::new(wall.start.x, wall.start.z);
                    let b = Vec2::new(wall.end.x, wall.end.z);
                    let closest = closest_point_on_segment_2d(p, a, b);
                    let delta = p - closest;
                    let distance = delta.length();
                    if distance < wall_padding {
                        let normal = if distance > 0.0001 {
                            delta / distance
                        } else {
                            let ab = b - a;
                            let n = Vec2::new(-ab.y, ab.x);
                            if n.length() > 0.0001 {
                                n.normalize()
                            } else {
                                Vec2::new(1.0, 0.0)
                            }
                        };
                        let push = normal * (wall_padding - distance);
                        compartment.position.x += push.x;
                        compartment.position.z += push.y;

                        let velocity = Vec2::new(compartment.velocity.x, compartment.velocity.z);
                        let normal_velocity = normal * velocity.dot(normal);
                        let tangent_velocity = velocity - normal_velocity;
                        let reflected_velocity = tangent_velocity - normal_velocity * 0.5;
                        compartment.velocity.x = reflected_velocity.x;
                        compartment.velocity.z = reflected_velocity.y;
                    }
                }

                compartment.acceleration = Vec3::new(0.0, 0.0, 0.0);
                compartment.influence = 0.0;
            }

            let mut next_compartments = compartments.clone();

            for ((key_a, compartment_a), (_next_key_a, next_a)) in
                compartments.iter().zip(&mut next_compartments)
            {
                for (key_b, compartment_b) in compartments.iter() {
                    if key_a == key_b {
                        continue;
                    }
                    let position_a = if compartment_a.position == compartment_b.position
                        || (compartment_a.position - compartment_b.position).length() < 0.1 * sigma
                    {
                        let offset = Vec3::new(0.1 * sigma, 0.0, 0.0);
                        if key_a < key_b {
                            compartment_a.position + offset
                        } else {
                            compartment_a.position - offset
                        }
                    } else {
                        compartment_a.position
                    };
                    let position_b = compartment_b.position;
                    let force = lennard_jones(position_a, position_b, eps, sigma);

                    next_a.acceleration += force;
                    next_a.influence += 0.01 * force.length();

                    let distance = (compartment_b.position - compartment_a.position).length();
                    if distance < connection_distance {
                        bonds.push(BondData {
                            position_a: compartment_a.position,
                            position_b: compartment_b.position,
                            strength: 0.5,
                            _padding: 0.0,
                        });
                    }
                }
            }
            for (_key, compartment) in &mut next_compartments {
                compartment.acceleration /= 1.0 + compartment.influence;
                compartment.acceleration += -0.5 * compartment.velocity;
            }

            *compartments = next_compartments;
        }

        self.particles = self
            .compartments
            .values()
            .map(|c| Particle {
                position: c.position,
                voltage: 0.0,
            })
            .collect();

        self.particle_buffer
            .update(&application.device, &application.queue, &self.particles);
        self.settings_buffer
            .update(&application.queue, &self.settings);
        self.lines_buffer
            .update(&application.device, &application.queue, &bonds);
    }

    fn render(&mut self, data: &mut RenderData) {
        self.spheres.render(data);
        self.lines.render(data);
        self.wall_lines.render(data);
    }

    fn gui(&mut self, _application: &visula::Application, context: &egui::Context) {
        egui::Window::new("Settings").show(context, |ui| {
            ui.label("Simulation speed");
            ui.add(egui::Slider::new(&mut self.settings.speed, 1..=20));
            ui.label("Radius");
            ui.add(egui::Slider::new(&mut self.settings.radius, 1.0..=20.0));
            ui.label("Width");
            ui.add(egui::Slider::new(&mut self.settings.width, 1.0..=20.0));
        });
    }

    fn handle_event(&mut self, application: &mut visula::Application, event: &Event<CustomEvent>) {
        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: MouseButton::Left,
                        ..
                    },
                ..
            } => {
                let position = match self.mouse.position {
                    Some(p) => p,
                    None => {
                        return;
                    }
                };
                let screen_position = Vec4::new(
                    2.0 * position.x as f32 / application.config.width as f32 - 1.0,
                    1.0 - 2.0 * position.y as f32 / application.config.height as f32,
                    1.0,
                    1.0,
                );
                let ray_clip = Vec4::new(screen_position.x, screen_position.y, -1.0, 1.0);
                let aspect_ratio =
                    application.config.width as f32 / application.config.height as f32;
                let inv_projection = application
                    .camera_controller
                    .projection_matrix(aspect_ratio)
                    .inverse();

                let ray_eye = inv_projection * ray_clip;
                let ray_eye = Vec4::new(ray_eye.x, ray_eye.y, -1.0, 0.0);
                let inv_view_matrix = application.camera_controller.view_matrix().inverse();
                let ray_world = inv_view_matrix * ray_eye;
                let ray_world = Vec3::new(ray_world.x, ray_world.y, ray_world.z).normalize();
                let ray_origin = application.camera_controller.position();
                let t = -ray_origin.y / ray_world.y;
                let intersection = ray_origin + t * ray_world;
                let intersection = Vec3::new(intersection.x, intersection.y, intersection.z);
                self.compartments.insert(Compartment {
                    position: intersection,
                    velocity: Vec3::new(0.0, 0.0, 0.0),
                    acceleration: Vec3::new(0.0, 0.0, 0.0),
                    influence: 0.0,
                    _padding: Default::default(),
                });
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                self.mouse.position = Some(*position);
            }

            _ => {}
        }
    }
}

fn main() {
    let args = Args::parse();
    let walls = load_walls_from_dxf(&args.dxf_path);

    visula::run(move |app| Simulation::new(app, walls.clone()).expect("Initializing simulation failed"));
}
