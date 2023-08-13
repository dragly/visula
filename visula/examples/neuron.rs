use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, SquareMatrix};
use hecs::World;

use glam::Vec3;
use visula::{
    CustomEvent, Expression, InstanceBuffer, LineDelegate, Lines, RenderData, SphereDelegate,
    Spheres, UniformBuffer,
};
use visula_derive::{Instance, Uniform};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, WindowEvent},
};

#[derive(Clone, Copy, Debug, Instance)]
struct Position {
    position: Vec3,
}

#[derive(Clone, Copy, Debug, Instance)]
struct Kinetic {
    velocity: Vec3,
    acceleration: Vec3,
    influence: f32,
}

#[derive(Clone, Copy, Debug, Instance)]
struct Compartment {
    voltage: f32,
    m: f32,
    h: f32,
    n: f32,
    capacitance: f32,
}

#[derive(Clone, Debug)]
struct Stimulator {
    position: Vec3,
    trigger: f32,
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
    mouse: Mouse,
    world: World,
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Result<Simulation, Error> {
        let particle_buffer = InstanceBuffer::<Particle>::new(&application.device);
        let particle = particle_buffer.instance();

        let lines_buffer = InstanceBuffer::<BondData>::new(&application.device);
        let bond = lines_buffer.instance();

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
                width: settings.width,
                alpha: bond.strength,
            },
        )
        .unwrap();

        let mut world = World::new();
        world.spawn((
            Position {
                position: Vec3::new(-20.0, 0.0, 0.0),
            },
            Compartment {
                voltage: 4.0266542,
                m: 0.084073044,
                h: 0.45317015,
                n: 0.38079754,
                capacitance: 4.0,
            },
        ));
        world.spawn((
            Position {
                position: Vec3::new(0.0, 0.0, -20.0),
            },
            Compartment {
                voltage: 4.0266542,
                m: 0.084073044,
                h: 0.45317015,
                n: 0.38079754,
                capacitance: 4.0,
            },
        ));
        world.spawn((
            Position {
                position: Vec3::new(20.0, 0.0, 20.0),
            },
            Compartment {
                voltage: 4.0266542,
                m: 0.084073044,
                h: 0.45317015,
                n: 0.38079754,
                capacitance: 4.0,
            },
        ));

        world.spawn((Stimulator {
            position: Vec3::new(260.0, 0.0, 0.0),
            trigger: 2.0,
        },));
        world.spawn((Stimulator {
            position: Vec3::new(0.0, 260.0, 0.0),
            trigger: 4.0,
        },));
        world.spawn((Stimulator {
            position: Vec3::new(0.0, 0.0, 260.0),
            trigger: 8.0,
        },));

        Ok(Simulation {
            particles: vec![],
            spheres,
            particle_buffer,
            lines_buffer,
            lines,
            settings: settings_data,
            settings_buffer,
            mouse: Mouse { position: None },
            world,
        })
    }
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn update(&mut self, application: &visula::Application) {
        let dt = 0.01;
        let node_radius = 15.0;
        let connection_distance = 5.0 * node_radius;
        let sigma = 3.0 * node_radius;
        let eps = 1.4;
        let max_velocity = node_radius * 2.0 / 3.0;
        let min_velocity = node_radius * 0.05;
        let mut bonds = vec![];
        for _ in 0..self.settings.speed {
            let stimulators: Vec<Stimulator> = self
                .world
                .query::<&Stimulator>()
                .iter()
                .map(|(_, s)| s.clone())
                .collect();
            for (_key, (position, kinetic)) in
                self.world.query_mut::<(&mut Position, &mut Kinetic)>()
            {
                kinetic.velocity += kinetic.acceleration * dt;
                if kinetic.velocity.length() > max_velocity {
                    kinetic.velocity = kinetic.velocity / kinetic.velocity.length() * max_velocity;
                }
                if kinetic.velocity.length() < min_velocity {
                    kinetic.velocity *= 0.0;
                }
                position.position += kinetic.velocity * dt;
                kinetic.acceleration = Vec3::new(0.0, 0.0, 0.0);
                kinetic.influence = 0.0;
            }
            for (_key, (position, compartment)) in
                self.world.query_mut::<(&Position, &mut Compartment)>()
            {
                let v = compartment.voltage;

                let sodium_activation_alpha = 0.1 * (25.0 - v) / ((2.5 - 0.1 * v).exp() - 1.0);
                let sodium_activation_beta = 4.0 * (-v / 18.0).exp();
                let sodium_inactivation_alpha = 0.07 * (-v / 20.0).exp();
                let sodium_inactivation_beta = 1.0 / ((3.0 - 0.1 * v).exp() + 1.0);

                let mut m = compartment.m;
                let alpham = sodium_activation_alpha;
                let betam = sodium_activation_beta;
                let dm = dt * (alpham * (1.0 - m) - betam * m);
                let mut h = compartment.h;
                let alphah = sodium_inactivation_alpha;
                let betah = sodium_inactivation_beta;
                let dh = dt * (alphah * (1.0 - h) - betah * h);

                m += dm;
                h += dh;

                m = m.clamp(0.0, 1.0);
                h = h.clamp(0.0, 1.0);

                let g_na = 120.0;

                let ena = 115.0;

                let m3 = m * m * m;

                let sodium_current = -g_na * m3 * h * (compartment.voltage - ena);

                let potassium_activation_alpha =
                    0.01 * (10.0 - v) / ((1.0 - (0.1 * v)).exp() - 1.0);
                let potassium_activation_beta = 0.125 * (-v / 80.0).exp();

                let mut n = compartment.n;
                let alphan = potassium_activation_alpha;
                let betan = potassium_activation_beta;
                let dn = dt * (alphan * (1.0 - n) - betan * n);

                n += dn;
                n = n.clamp(0.0, 1.0);

                let g_k = 36.0;
                let ek = -12.0;
                let n4 = n * n * n * n;

                let potassium_current = -g_k * n4 * (compartment.voltage - ek);

                let e_m = 10.6;
                let leak_conductance = 1.3;
                let leak_current = -leak_conductance * (compartment.voltage - e_m);

                let mut injected_current = 0.0;
                for stimulator in &stimulators {
                    let distance = position.position.distance(stimulator.position);
                    if distance < sigma && stimulator.trigger < 0.0 {
                        injected_current += 50000.0;
                    }
                }

                let current = sodium_current + potassium_current + leak_current + injected_current;
                let delta_voltage = current / compartment.capacitance;

                compartment.n = n;
                compartment.m = m;
                compartment.h = h;
                compartment.voltage += delta_voltage * dt;
                compartment.voltage = compartment.voltage.clamp(-50.0, 200.0)
            }

            let compartments: Vec<(Position, Kinetic, Compartment)> = self
                .world
                .query::<(&Position, &Kinetic, &Compartment)>()
                .iter()
                .map(|(_, (p, k, c))| (p.clone(), k.clone(), c.clone()))
                .collect();
            let mut next_compartments = compartments.clone();

            for (
                (key_a, (position_a, kinetic_a, compartment_a)),
                (_next_key_a, (next_position_a, next_kinetic_a, next_compartment_a)),
            ) in compartments
                .iter()
                .enumerate()
                .zip(next_compartments.iter_mut().enumerate())
            {
                for (key_b, (position_b, kinetic_b, compartment_b)) in
                    compartments.iter().enumerate()
                {
                    if key_a == key_b {
                        continue;
                    }
                    let position_a = if position_a.position == position_b.position
                        || (position_a.position - position_b.position).length() < 0.1 * sigma
                    {
                        let offset = Vec3::new(0.1 * sigma, 0.0, 0.0);
                        if key_a < key_b {
                            position_a.position + offset
                        } else {
                            position_a.position - offset
                        }
                    } else {
                        position_a.position
                    };
                    let position_b = position_b.position;
                    let force = lennard_jones(position_a, position_b, eps, sigma);

                    next_kinetic_a.acceleration += force;
                    next_kinetic_a.influence += 0.01 * force.length();

                    let distance = (position_b - position_a).length();
                    if distance < connection_distance {
                        let voltage_diff = compartment_b.voltage - compartment_a.voltage;
                        let delta_voltage = voltage_diff / compartment_a.capacitance;
                        next_compartment_a.voltage += delta_voltage * dt;
                        let value = voltage_diff.abs() * 0.01;
                        bonds.push(BondData {
                            position_a,
                            position_b,
                            strength: 0.5 + value,
                            _padding: 0.0,
                        });
                    }
                }
            }

            for (position, kinetic, compartment) in &mut next_compartments {
                kinetic.acceleration /= 1.0 + kinetic.influence;
                kinetic.acceleration += -0.5 * kinetic.velocity;
            }

            for (
                (_, (position, kinetic, compartment)),
                (next_position, next_kinetic, next_compartment),
            ) in self
                .world
                .query_mut::<(&mut Position, &mut Kinetic, &mut Compartment)>()
                .into_iter()
                .zip(next_compartments)
            {
                *position = next_position;
                *kinetic = next_kinetic;
                *compartment = next_compartment;
            }

            let compartments: Vec<(Position, Compartment)> = self
                .world
                .query::<(&Position, &Compartment)>()
                .iter()
                .map(|(_, (p, c))| (p.clone(), c.clone()))
                .collect();
            let mut next_compartments = compartments.clone();

            for (
                (key_a, (position_a, compartment_a)),
                (_next_key_a, (next_position_a, next_compartment_a)),
            ) in compartments
                .iter()
                .enumerate()
                .zip(next_compartments.iter_mut().enumerate())
            {
                for (key_b, (position_b, compartment_b)) in compartments.iter().enumerate() {
                    let distance = (position_b.position - position_a.position).length();
                    if distance < connection_distance {
                        let voltage_diff = compartment_b.voltage - compartment_a.voltage;
                        let delta_voltage = voltage_diff / compartment_a.capacitance;
                        next_compartment_a.voltage += delta_voltage * dt;
                        let value = voltage_diff.abs() * 0.01;
                        bonds.push(BondData {
                            position_a: position_a.position,
                            position_b: position_b.position,
                            strength: 0.5 + value,
                            _padding: 0.0,
                        });
                    }
                }
            }

            for ((_, (position, compartment)), (next_position, next_compartment)) in self
                .world
                .query_mut::<(&mut Position, &mut Compartment)>()
                .into_iter()
                .zip(next_compartments)
            {
                *position = next_position;
                *compartment = next_compartment;
            }

            for (_entity, stimulator) in self.world.query_mut::<&mut Stimulator>() {
                stimulator.trigger = if stimulator.trigger < 0.0 {
                    16.0
                } else {
                    stimulator.trigger - dt
                };
            }
        }

        self.particles = self
            .world
            .query::<(&Position, &Compartment)>()
            .iter()
            .map(|(_, (p, c))| Particle {
                position: p.position,
                voltage: c.voltage,
            })
            .collect();

        self.particles.extend(
            self.world
                .query::<&Stimulator>()
                .iter()
                .map(|(_, s)| Particle {
                    position: s.position,
                    voltage: s.trigger * 10.0,
                }),
        );

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
    }

    fn gui(&mut self, context: &egui::Context) {
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
                        button,
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
                let screen_position = cgmath::Vector4 {
                    x: 2.0 * position.x as f32 / application.config.width as f32 - 1.0,
                    y: 1.0 - 2.0 * position.y as f32 / application.config.height as f32,
                    z: 1.0,
                    w: 1.0,
                };
                let ray_clip = cgmath::Vector4 {
                    x: screen_position.x,
                    y: screen_position.y,
                    z: -1.0,
                    w: 1.0,
                };
                let aspect_ratio =
                    application.config.width as f32 / application.config.height as f32;
                let inv_projection = application
                    .camera_controller
                    .projection_matrix(aspect_ratio)
                    .invert()
                    .unwrap();

                let ray_eye = inv_projection * ray_clip;
                let ray_eye = cgmath::Vector4 {
                    x: ray_eye.x,
                    y: ray_eye.y,
                    z: -1.0,
                    w: 0.0,
                };
                let inv_view_matrix = application
                    .camera_controller
                    .view_matrix()
                    .invert()
                    .unwrap();
                let ray_world = inv_view_matrix * ray_eye;
                let ray_world = cgmath::Vector3 {
                    x: ray_world.x,
                    y: ray_world.y,
                    z: ray_world.z,
                }
                .normalize();
                let ray_origin = application.camera_controller.position();
                let mut command_buffer = hecs::CommandBuffer::new();
                self.world
                    .query::<&Position>()
                    .iter()
                    .filter_map(|(entity, compartment)| {
                        let position = cgmath::Point3 {
                            x: compartment.position.x,
                            y: compartment.position.y,
                            z: compartment.position.z,
                        };
                        let v = position - ray_origin;
                        let t = v.dot(ray_world);
                        let r = ray_origin + t * ray_world;
                        let d = r - position;
                        let distance = d.magnitude();
                        if distance > self.settings.radius * 10.0 {
                            return None;
                        }
                        let placement = position + 5.0 * self.settings.radius * d.normalize();
                        Some((entity, placement, distance))
                    })
                    .min_by(|(_, _, a), (_, _, b)| {
                        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(entity, placement, _)| {
                        if *button == MouseButton::Left {
                            command_buffer.spawn((
                                Position {
                                    position: Vec3::new(placement.x, placement.y, placement.z),
                                },
                                Kinetic {
                                    velocity: Vec3::new(0.0, 0.0, 0.0),
                                    acceleration: Vec3::new(0.0, 0.0, 0.0),
                                    influence: 0.0,
                                },
                                Compartment {
                                    voltage: 100.0,
                                    m: 0.084073044,
                                    h: 0.45317015,
                                    n: 0.38079754,
                                    capacitance: 4.0,
                                },
                            ));
                        } else {
                            command_buffer.despawn(entity);
                        }
                    });
                command_buffer.run_on(&mut self.world);
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
    visula::run(|app| Simulation::new(app).expect("Initializing simulation failed"));
}
