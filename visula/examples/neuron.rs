use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, SquareMatrix};
use slotmap::{DefaultKey, SlotMap};

use glam::Vec3;
use visula::{
    simulation::RenderData, BindingBuilder, Buffer, BufferBinding, BufferBindingField,
    BufferInner, CustomEvent, Expression, Instance, InstanceField, InstanceHandle, LineDelegate,
    Lines, NagaType, SphereDelegate, Spheres, Uniform, UniformBinding, UniformField, UniformHandle,
    VertexAttrFormat, VertexBufferLayoutBuilder,
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
    voltage: f32,
    m: f32,
    h: f32,
    n: f32,
    influence: f32,
    capacitance: f32,
    _padding: f32,
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
    particle_buffer: Buffer<Particle>,
    lines: Lines,
    settings: Settings,
    settings_buffer: Buffer<Settings>,
    lines_buffer: Buffer<BondData>,
    compartments: SlotMap<DefaultKey, Compartment>,
    mouse: Mouse,
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn init(application: &mut visula::Application) -> Result<Simulation, Error> {
        let particle_buffer = Buffer::<Particle>::new(application);
        let particle = particle_buffer.instance();

        let lines_buffer = Buffer::<BondData>::new(application);
        let bond = lines_buffer.instance();

        let settings_data = Settings {
            radius: 10.0,
            width: 4.0,
            speed: 4,
            //_padding: 0.0,
        };
        let settings_buffer = Buffer::new_with_init(application, &[settings_data]);
        let settings = settings_buffer.uniform();
        let pos = &particle.position;
        let spheres = Spheres::new(
            application,
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
            application,
            &LineDelegate {
                start: bond.position_a,
                end: bond.position_b,
                width: settings.width,
                alpha: bond.strength,
            },
        )
        .unwrap();

        let mut compartments = SlotMap::<DefaultKey, Compartment>::new();
        compartments.insert(Compartment {
            position: Vec3::new(0.0, 0.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            acceleration: Vec3::new(0.0, 0.0, 0.0),
            voltage: 4.0266542,
            m: 0.084073044,
            h: 0.45317015,
            n: 0.38079754,
            influence: 0.0,
            capacitance: 4.0,
            _padding: Default::default(),
        });
        compartments.insert(Compartment {
            position: Vec3::new(2.0, 0.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            acceleration: Vec3::new(0.0, 0.0, 0.0),
            voltage: 4.0266542,
            m: 0.084073044,
            h: 0.45317015,
            n: 0.38079754,
            influence: 0.0,
            capacitance: 4.0,
            _padding: Default::default(),
        });
        compartments.insert(Compartment {
            position: Vec3::new(2.0, 0.0, 2.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            acceleration: Vec3::new(0.0, 0.0, 0.0),
            voltage: 4.0266542,
            m: 0.084073044,
            h: 0.45317015,
            n: 0.38079754,
            influence: 0.0,
            capacitance: 4.0,
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
            compartments,
            mouse: Mouse { position: None },
        })
    }

    fn update(&mut self, application: &visula::Application) {
        let compartments = &mut self.compartments;
        let injecting_current = false;
        let mouse = Vec3::new(0.0, 0.0, 0.0);
        let dt = 0.01;
        let node_radius = 15.0;
        let connection_distance = 5.0 * node_radius;
        let sigma = 3.0 * node_radius;
        let eps = 1.4;
        let max_velocity = node_radius * 2.0 / 3.0;
        let min_velocity = node_radius * 0.05;
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
                compartment.acceleration = Vec3::new(0.0, 0.0, 0.0);
                compartment.influence = 0.0;

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

                let injected_current = if injecting_current {
                    let mouse_distance = (compartment.position - mouse).length();
                    150.0 * (-mouse_distance * mouse_distance / (2.0 * sigma * sigma)).exp()
                } else {
                    0.0
                };
                let current = sodium_current + potassium_current + leak_current + injected_current;
                let delta_voltage = current / compartment.capacitance;

                compartment.n = n;
                compartment.m = m;
                compartment.h = h;
                compartment.voltage += delta_voltage * dt;
                compartment.voltage = compartment.voltage.clamp(-50.0, 200.0)
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
                        let voltage_diff = compartment_b.voltage - compartment_a.voltage;
                        let delta_voltage = voltage_diff / compartment_a.capacitance;
                        next_a.voltage += delta_voltage * dt;
                        let value = voltage_diff.abs() * 0.01;
                        bonds.push(BondData {
                            position_a: compartment_a.position,
                            position_b: compartment_b.position,
                            strength: 0.5 + value,
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
                voltage: c.voltage,
            })
            .collect();

        self.particle_buffer.update(application, &self.particles);
        self.settings_buffer.update(application, &[self.settings]);
        self.lines_buffer.update(application, &bonds);
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
                let t = -ray_origin.y / ray_world.y;
                let intersection = ray_origin + t * ray_world;
                let intersection = Vec3::new(intersection.x, intersection.y, intersection.z);
                self.compartments.insert(Compartment {
                    position: intersection,
                    velocity: Vec3::new(0.0, 0.0, 0.0),
                    acceleration: Vec3::new(0.0, 0.0, 0.0),
                    //voltage: 4.0266542,
                    voltage: 100.0,
                    m: 0.084073044,
                    h: 0.45317015,
                    n: 0.38079754,
                    influence: 0.0,
                    capacitance: 4.0,
                    _padding: 0.0,
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
    visula::run::<Simulation>();
}
