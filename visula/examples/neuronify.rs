use std::collections::HashMap;
use std::f32::consts::PI;
use std::io::Cursor;
use std::iter::FromIterator;
use visula::Renderable;

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Quat, Vec3, Vec4};
use hecs::Entity;
use itertools::Itertools;
use rand::Rng;
use strum::EnumIter;
use strum::IntoEnumIterator;
use visula::io::gltf::{parse_gltf, GltfMesh};
use visula::{
    CustomEvent, InstanceBuffer, LineDelegate, Lines, MeshDelegate, MeshPipeline, RenderData,
    SphereDelegate, Spheres,
};
use visula_derive::Instance;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, WindowEvent},
};

const BOUNDARY: f32 = 20.0;

#[derive(Clone, Debug, EnumIter, PartialEq)]
pub enum Tool {
    Select,
    ExcitatoryNeuron,
    InhibitoryNeuron,
    StaticConnection,
    LearningConnection,
    Erase,
    Stimulate,
}

const NODE_RADIUS: f32 = 1.0;
const ERASE_RADIUS: f32 = 2.0 * NODE_RADIUS;
const SIGMA: f32 = 3.0 * NODE_RADIUS;

#[derive(Clone, Debug)]
enum NeuronType {
    Excitatory,
    Inhibitory,
}

#[derive(Clone, Debug)]
struct NeuronDynamics {
    voltage: f64,
    input_current: f64,
    refraction: f64,
    fired: bool,
    last_fired: Option<f64>,
}

#[derive(Clone, Debug)]
struct Neuron {
    initial_voltage: f64,
    resting_potential: f64,
    leak_tau: f64,
    threshold: f64,
    input_tau: f64,
    ty: NeuronType,
    dynamics: HashMap<Entity, NeuronDynamics>,
}

#[derive(Clone, Debug)]
struct Boid {
    velocity: Vec3,
    angular_velocity: f32,
}

#[derive(Clone, Debug)]
struct Selectable {
    selected: bool,
}

struct LearningSynapse {}

struct PreviousCreation {
    position: Vec3,
}

#[derive(Clone, Debug)]
struct Deletable {}

#[derive(Clone, Debug)]
pub struct Position {
    pub position: Vec3,
}

impl Neuron {
    pub fn new(ty: NeuronType, boids: &[Entity]) -> Neuron {
        let dynamics = HashMap::from_iter(boids.iter().map(|e| {
            (
                *e,
                NeuronDynamics {
                    refraction: 0.0,
                    voltage: 0.0,
                    input_current: 0.0,
                    fired: false,
                    last_fired: None,
                },
            )
        }));
        Neuron {
            initial_voltage: 0.0,
            leak_tau: 1.0,
            resting_potential: -70.0,
            threshold: 30.0,
            input_tau: 0.1,
            ty,
            dynamics,
        }
    }
}

#[derive(Clone, Debug)]
struct ConnectionTool {
    start: Vec3,
    end: Vec3,
    from: Entity,
}

#[derive(Clone, Debug)]
struct Trigger {
    total: f64,
    remaining: f64,
    current: f64,
    connection: Entity,
    boid: Entity,
}

impl Trigger {
    pub fn new(time: f64, current: f64, connection: Entity, boid: Entity) -> Trigger {
        Trigger {
            total: time,
            remaining: time,
            current,
            connection,
            boid,
        }
    }
    pub fn decrement(&mut self, dt: f64) {
        self.remaining = (self.remaining - dt).max(0.0);
    }
    pub fn progress(&self) -> f64 {
        (self.total - self.remaining) / self.total
    }
    pub fn done(&self) -> bool {
        self.remaining <= 0.0
    }
}

#[derive(Clone, Debug)]
struct Connection {
    from: Entity,
    to: Entity,
    strength: f64,
}

#[derive(Clone, Debug)]
pub struct Stimulate {
    pub injected_current: f64,
}

impl Stimulate {
    pub fn new() -> Stimulate {
        Stimulate {
            injected_current: 0.0,
        }
    }
}

impl Default for Stimulate {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
struct StimulationTool {
    position: Vec3,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Instance, Pod, Zeroable)]
struct Sphere {
    position: glam::Vec3,
    color: glam::Vec3,
    _padding: [f32; 2],
}

struct Mouse {
    left_down: bool,
    position: Option<PhysicalPosition<f64>>,
}

struct Simulation {
    boids: Vec<Entity>,
    selected_boid: Entity,
    tool: Tool,
    previous_creation: Option<PreviousCreation>,
    connection_tool: Option<ConnectionTool>,
    stimulation_tool: Option<StimulationTool>,
    world: hecs::World,
    time: f64,
    mouse: Mouse,
    spheres: Spheres,
    sphere_buffer: InstanceBuffer<Sphere>,
    mesh_instance_buffer: InstanceBuffer<MeshInstanceData>,
    lines: Lines,
    line_buffer: InstanceBuffer<BondData>,
    boundaries: Lines,
    iterations: u32,
    mesh: MeshPipeline,
}

#[derive(Debug)]
struct Error {}

#[derive(Clone, Debug)]
enum Input {
    Repulsion { min_angle: f32, max_angle: f32 },
    Attraction { min_angle: f32, max_angle: f32 },
    Alignment { min_angle: f32, max_angle: f32 },
}

#[derive(Clone, Debug)]
enum Output {
    TurnLeft,
    TurnRight,
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
struct LineData {
    start: Vec3,
    end: Vec3,
    _padding: [f32; 2],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct MeshInstanceData {
    position: Vec3,
    _padding: f32,
    rotation: Quat,
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Simulation {
        application.camera_controller.enabled = false;

        let sphere_buffer = InstanceBuffer::<Sphere>::new(&application.device);
        let line_buffer = InstanceBuffer::<BondData>::new(&application.device);
        let mesh_instance_buffer = InstanceBuffer::<MeshInstanceData>::new(&application.device);
        let sphere = sphere_buffer.instance();
        let line = line_buffer.instance();

        let mesh_instance = mesh_instance_buffer.instance();
        let spheres = Spheres::new(
            &application.rendering_descriptor(),
            &SphereDelegate {
                position: sphere.position.clone(),
                radius: NODE_RADIUS.into(),
                color: sphere.color,
            },
        )
        .unwrap();
        let lines = Lines::new(
            &application.rendering_descriptor(),
            &LineDelegate {
                start: line.position_a.clone(),
                end: line.position_b,
                width: 0.2.into(),
                color: glam::Vec3::new(1.0, 0.8, 1.0).into(),
            },
        )
        .unwrap();

        let boundary_buffer = InstanceBuffer::<LineData>::new(&application.device);
        let boundary = boundary_buffer.instance();
        let boundaries = Lines::new(
            &application.rendering_descriptor(),
            &LineDelegate {
                start: boundary.start,
                end: boundary.end,
                width: 0.3.into(),
                color: glam::Vec3::new(1.0, 0.8, 1.0).into(),
            },
        )
        .unwrap();

        let boundary_data = [
            LineData {
                start: Vec3::new(-BOUNDARY, 0.0, -BOUNDARY),
                end: Vec3::new(BOUNDARY, 0.0, -BOUNDARY),
                _padding: Default::default(),
            },
            LineData {
                start: Vec3::new(-BOUNDARY, 0.0, -BOUNDARY),
                end: Vec3::new(-BOUNDARY, 0.0, BOUNDARY),
                _padding: Default::default(),
            },
            LineData {
                start: Vec3::new(BOUNDARY, 0.0, -BOUNDARY),
                end: Vec3::new(BOUNDARY, 0.0, BOUNDARY),
                _padding: Default::default(),
            },
            LineData {
                start: Vec3::new(-BOUNDARY, 0.0, BOUNDARY),
                end: Vec3::new(BOUNDARY, 0.0, BOUNDARY),
                _padding: Default::default(),
            },
        ];

        boundary_buffer.update(&application.device, &application.queue, &boundary_data);

        let mut reader = Cursor::new(include_bytes!("./boid.glb"));
        let gltf_file = parse_gltf(&mut reader, application).expect("Could not parse GLTF");

        let mut mesh = MeshPipeline::new(
            &application.rendering_descriptor(),
            &MeshDelegate {
                position: mesh_instance.position,
                rotation: mesh_instance.rotation,
                scale: Vec3::ONE.into(),
            },
        )
        .unwrap();
        let GltfMesh {
            vertex_buffer,
            index_buffer,
            index_count,
        } = gltf_file
            .scenes
            .into_iter()
            .next()
            .unwrap()
            .meshes
            .into_iter()
            .next()
            .unwrap();
        mesh.vertex_count = index_count;
        mesh.vertex_buffer = vertex_buffer;
        mesh.index_buffer = index_buffer;

        let mut world = hecs::World::new();

        let mut rng = rand::rng();
        let mut boids = vec![];
        for _ in 0..50 {
            boids.push(world.spawn((
                Position {
                    position: 20.0 * (1.0 - 2.0 * Vec3::new(rng.random(), 0.5, rng.random())),
                },
                Boid {
                    velocity: 5.0 * Vec3::new(rng.random(), 0.0, rng.random()).normalize(),
                    angular_velocity: 2.0 * rng.random::<f32>() - 1.0,
                },
            )));
        }
        let player_boid = *boids.first().expect("No boids were added!");

        let angles = [-140.0f32, -100.0, -60.0, 0.0, 60.0, 100.0, 140.0];
        for angle_pair in angles.windows(2) {
            if let &[min_angle, max_angle] = angle_pair {
                let average = (min_angle + max_angle) / 2.0;
                world.spawn((
                    Position {
                        position: Vec3::new(0.1 * average, 0.0, -32.0),
                    },
                    Neuron::new(NeuronType::Excitatory, &boids),
                    Stimulate::new(),
                    Input::Repulsion {
                        min_angle,
                        max_angle,
                    },
                ));
                world.spawn((
                    Position {
                        position: Vec3::new(-0.1 * average, 0.0, -28.0),
                    },
                    Neuron::new(NeuronType::Excitatory, &boids),
                    Stimulate::new(),
                    Input::Attraction {
                        min_angle,
                        max_angle,
                    },
                ));
                world.spawn((
                    Position {
                        position: Vec3::new(0.1 * average, 0.0, -23.0),
                    },
                    Neuron::new(NeuronType::Excitatory, &boids),
                    Stimulate::new(),
                    Input::Alignment {
                        min_angle,
                        max_angle,
                    },
                ));
            }
        }

        world.spawn((
            Position {
                position: Vec3::new(-15.0, 0.0, -20.0),
            },
            Stimulate::new(),
            Neuron::new(NeuronType::Excitatory, &boids),
            Output::TurnLeft {},
        ));
        world.spawn((
            Position {
                position: Vec3::new(15.0, 0.0, -20.0),
            },
            Stimulate::new(),
            Neuron::new(NeuronType::Excitatory, &boids),
            Output::TurnRight {},
        ));

        Simulation {
            boids,
            selected_boid: player_boid,
            spheres,
            sphere_buffer,
            lines,
            line_buffer,
            boundaries,
            tool: Tool::ExcitatoryNeuron,
            previous_creation: None,
            connection_tool: None,
            stimulation_tool: None,
            world,
            time: 0.0,
            mouse: Mouse {
                left_down: false,
                position: None,
            },
            iterations: 4,
            mesh,
            mesh_instance_buffer,
        }
    }

    fn handle_tool(&mut self, application: &visula::Application) {
        let Simulation {
            tool,
            mouse,
            connection_tool,
            stimulation_tool,
            world,
            previous_creation,
            ..
        } = self;
        if !mouse.left_down {
            *stimulation_tool = None;
            *connection_tool = None;
            *previous_creation = None;
            return;
        }
        let mouse_physical_position = match mouse.position {
            Some(p) => p,
            None => {
                return;
            }
        };
        let screen_position = Vec4::new(
            2.0 * mouse_physical_position.x as f32 / application.config.width as f32 - 1.0,
            1.0 - 2.0 * mouse_physical_position.y as f32 / application.config.height as f32,
            1.0,
            1.0,
        );
        let ray_clip = Vec4::new(screen_position.x, screen_position.y, -1.0, 1.0);
        let aspect_ratio = application.config.width as f32 / application.config.height as f32;
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
        let mouse_position = Vec3::new(intersection.x, intersection.y, intersection.z);

        let minimum_distance = 6.0 * NODE_RADIUS;
        match tool {
            Tool::ExcitatoryNeuron | Tool::InhibitoryNeuron => {
                if previous_creation.is_none()
                    || previous_creation
                        .as_ref()
                        .unwrap()
                        .position
                        .distance(mouse_position)
                        > minimum_distance
                {
                    match self.tool {
                        Tool::ExcitatoryNeuron => {
                            self.world.spawn((
                                Position {
                                    position: mouse_position,
                                },
                                Neuron::new(NeuronType::Excitatory, &self.boids),
                                Deletable {},
                                Stimulate::new(),
                                Selectable { selected: false },
                            ));
                        }
                        Tool::InhibitoryNeuron => {
                            self.world.spawn((
                                Position {
                                    position: mouse_position,
                                },
                                Neuron::new(NeuronType::Inhibitory, &self.boids),
                                Deletable {},
                                Stimulate::new(),
                                Selectable { selected: false },
                            ));
                        }
                        _ => {}
                    }
                    self.previous_creation = Some(PreviousCreation {
                        position: mouse_position,
                    });
                }
            }
            Tool::StaticConnection | Tool::LearningConnection => {
                let nearest = world
                    .query::<&Position>()
                    .with::<&Neuron>()
                    .iter()
                    .min_by(|(_, x), (_, y)| {
                        mouse_position
                            .distance(x.position)
                            .partial_cmp(&mouse_position.distance(y.position))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .and_then(|(id, position)| {
                        if mouse_position.distance(position.position) < 1.5 * NODE_RADIUS {
                            Some((id, position.position))
                        } else {
                            None
                        }
                    });
                if let Some(ct) = connection_tool {
                    if let Some((id, position)) = nearest {
                        let strength = if *tool == Tool::StaticConnection {
                            1.0
                        } else {
                            0.0
                        };
                        let new_connection = Connection {
                            from: ct.from,
                            to: id,
                            strength,
                        };
                        let connection_exists =
                            world.query::<&Connection>().iter().any(|(_, c)| {
                                c.from == new_connection.from && c.to == new_connection.to
                            });
                        if !connection_exists && ct.from != id {
                            if *tool == Tool::StaticConnection {
                                world.spawn((new_connection, Deletable {}));
                            } else {
                                world.spawn((new_connection, Deletable {}, LearningSynapse {}));
                            };
                        }
                        ct.start = position;
                        ct.from = id;
                    }
                    ct.end = mouse_position;
                } else if let Some((id, position)) = nearest {
                    *connection_tool = Some(ConnectionTool {
                        start: position,
                        end: mouse_position,
                        from: id,
                    });
                }
            }
            Tool::Stimulate => {
                *stimulation_tool = Some(StimulationTool {
                    position: mouse_position,
                })
            }
            Tool::Erase => {
                let to_delete = world
                    .query::<&Position>()
                    .with::<&Deletable>()
                    .iter()
                    .filter_map(|(entity, position)| {
                        let distance = position.position.distance(mouse_position);
                        if distance < NODE_RADIUS * 1.5 {
                            Some(entity)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<Entity>>();
                for entity in to_delete {
                    world.despawn(entity).unwrap();
                }
                let connections_to_delete = world
                    .query::<&Connection>()
                    .with::<&Deletable>()
                    .iter()
                    .filter_map(|(entity, connection)| {
                        if let (Ok(from), Ok(to)) = (
                            world.get::<&Position>(connection.from),
                            world.get::<&Position>(connection.to),
                        ) {
                            let a = from.position;
                            let b = to.position;
                            let p = mouse_position;
                            let ab = b - a;
                            let ap = p - a;
                            let t = ap.dot(ab) / ab.dot(ab);
                            let d = t * ab;
                            let point_on_line = a + d;
                            let distance_from_line = p.distance(point_on_line);
                            if distance_from_line < ERASE_RADIUS && (0.0..=1.0).contains(&t) {
                                Some(entity)
                            } else {
                                None
                            }
                        } else {
                            Some(entity)
                        }
                    })
                    .collect::<Vec<Entity>>();
                for connection in connections_to_delete {
                    world.despawn(connection).unwrap();
                }
                let triggers_to_delete = world
                    .query::<&Trigger>()
                    .iter()
                    .filter_map(|(entity, trigger)| {
                        if world.get::<&Connection>(trigger.connection).is_err() {
                            Some(entity)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<Entity>>();
                for trigger in triggers_to_delete {
                    world.despawn(trigger).unwrap();
                }
            }
            Tool::Select => {
                for (_, (selectable, position)) in world.query_mut::<(&mut Selectable, &Position)>()
                {
                    let distance = position.position.distance(mouse_position);
                    if distance < NODE_RADIUS {
                        selectable.selected = true;
                    }
                }
            }
        }
    }
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn update(&mut self, application: &mut visula::Application) {
        let Simulation {
            connection_tool,
            world,
            time,
            stimulation_tool,
            selected_boid,
            ..
        } = self;
        let dt = 0.001;

        for (_, (position, stimulate)) in world.query_mut::<(&Position, &mut Stimulate)>() {
            if let Some(stim) = stimulation_tool {
                let mouse_distance = position.position.distance(stim.position);
                stimulate.injected_current = (2000.0
                    * (-mouse_distance * mouse_distance / (2.0 * SIGMA * SIGMA)).exp())
                    as f64;
            } else {
                stimulate.injected_current = 0.0;
            }
        }

        for _ in 0..self.iterations {
            for (_, (neuron, stimulate)) in world.query_mut::<(&mut Neuron, &Stimulate)>() {
                for dynamics in neuron.dynamics.values_mut() {
                    dynamics.input_current =
                        dynamics.input_current - dynamics.input_current * dt / neuron.input_tau;
                    let leak_current =
                        (neuron.resting_potential - dynamics.voltage) / neuron.leak_tau;
                    let other_currents = if dynamics.refraction <= 0.0 {
                        dynamics.input_current + stimulate.injected_current
                    } else {
                        0.0
                    };
                    let current = leak_current + other_currents;
                    dynamics.voltage = (dynamics.voltage + current * dt).clamp(-200.0, 200.0);
                    if dynamics.refraction <= 0.0 && dynamics.voltage > neuron.threshold {
                        dynamics.fired = true;
                        dynamics.last_fired = Some(*time);
                        dynamics.refraction = 0.2;
                        dynamics.voltage = neuron.initial_voltage;
                        dynamics.voltage = neuron.resting_potential;
                    }
                }
            }
            let new_triggers: Vec<(Entity, Entity, f64)> = world
                .query::<&Connection>()
                .iter()
                .flat_map(|(connection_entity, connection)| {
                    let neuron_from = world
                        .get::<&Neuron>(connection.from)
                        .expect("Connection from does not exist!");
                    let base_current = match &neuron_from.ty {
                        NeuronType::Excitatory => 3000.0,
                        NeuronType::Inhibitory => -1500.0,
                    };
                    let current = connection.strength * base_current;
                    let mut triggers = vec![];
                    for (boid, dynamics) in neuron_from.dynamics.iter() {
                        if dynamics.fired {
                            triggers.push((connection_entity, *boid, current));
                        }
                    }
                    triggers
                })
                .collect();
            for (connection_entity, boid, current) in new_triggers {
                world.spawn((Trigger::new(0.01, current, connection_entity, boid),));
            }

            for (_, trigger) in world.query_mut::<&mut Trigger>() {
                trigger.decrement(dt);
            }

            let boids: Vec<(Entity, Position, Boid)> = world
                .query::<(&Position, &Boid)>()
                .iter()
                .map(|(e, (p, b))| (e, p.clone(), b.clone()))
                .collect();

            for ((entity_a, position_a, boid_a), (entity_b, position_b, boid_b)) in
                boids.iter().cartesian_product(boids.iter())
            {
                if entity_a == entity_b {
                    continue;
                }
                let diff = position_b.position - position_a.position;
                let up = Vec3::Y;
                let relative_angle = diff.cross(boid_a.velocity).dot(up).signum()
                    * diff.normalize().dot(boid_a.velocity.normalize()).acos()
                    * 180.0
                    / PI;
                let relative_alignment = boid_a.velocity.cross(boid_b.velocity).y.signum()
                    * boid_a
                        .velocity
                        .normalize()
                        .dot(boid_b.velocity.normalize())
                        .acos()
                    * 180.0
                    / PI;
                for (_entity, (input, neuron)) in world.query_mut::<(&Input, &mut Neuron)>() {
                    match *input {
                        Input::Repulsion {
                            min_angle,
                            max_angle,
                        } => {
                            if diff.length() < 6.0
                                && relative_angle > min_angle
                                && relative_angle < max_angle
                            {
                                neuron.dynamics.get_mut(entity_a).unwrap().input_current +=
                                    1.0 / diff.length() as f64 * 10000.0 * dt;
                            }
                        }
                        Input::Attraction {
                            min_angle,
                            max_angle,
                        } => {
                            if diff.length() > 6.0
                                && diff.length() < 12.0
                                && relative_angle > min_angle
                                && relative_angle < max_angle
                            {
                                neuron.dynamics.get_mut(entity_a).unwrap().input_current +=
                                    10000.0 * dt;
                            }
                        }
                        Input::Alignment {
                            min_angle,
                            max_angle,
                        } => {
                            if diff.length() < 12.0
                                && relative_alignment > min_angle
                                && relative_alignment < max_angle
                            {
                                neuron.dynamics.get_mut(entity_a).unwrap().input_current +=
                                    10000.0 * dt;
                            }
                        }
                    }
                }
            }

            let triggers_to_delete: Vec<Entity> = world
                .query::<&Trigger>()
                .iter()
                .filter_map(|(entity, trigger)| {
                    if trigger.done() {
                        let connection = world.get::<&Connection>(trigger.connection).unwrap();
                        let mut neuron_to = world.get::<&mut Neuron>(connection.to).unwrap();
                        neuron_to
                            .dynamics
                            .get_mut(&trigger.boid)
                            .unwrap()
                            .input_current += trigger.current;
                        neuron_to
                            .dynamics
                            .get_mut(&trigger.boid)
                            .unwrap()
                            .input_current = neuron_to.dynamics[&trigger.boid]
                            .input_current
                            .clamp(-20000.0, 20000.0);
                        Some(entity)
                    } else {
                        None
                    }
                })
                .collect();
            for entity in triggers_to_delete {
                world.despawn(entity).expect("Could not delete entity!");
            }

            let mut fired_outputs = vec![];
            for (_, (neuron, output)) in world.query_mut::<(&Neuron, &Output)>() {
                for (boid, dynamics) in &neuron.dynamics {
                    if dynamics.fired {
                        fired_outputs.push((*boid, output.clone()));
                    }
                }
            }

            for (target, output) in fired_outputs {
                let mut boid = world.get::<&mut Boid>(target).expect("Target not found");
                let turn_angle = match output {
                    Output::TurnLeft => -400.0,
                    Output::TurnRight => 400.0,
                };
                boid.angular_velocity = turn_angle;
            }

            for (_, neuron) in world.query_mut::<&mut Neuron>() {
                for dynamics in neuron.dynamics.values_mut() {
                    dynamics.fired = false;
                    dynamics.refraction -= dt;
                }
            }

            for (_entity, (boid, position)) in world.query_mut::<(&mut Boid, &mut Position)>() {
                position.position += boid.velocity * dt as f32;
                let rotation =
                    Mat3::from_rotation_y(boid.angular_velocity * dt as f32 / 180.0 * PI);
                boid.velocity = rotation * boid.velocity;
                boid.angular_velocity -= boid.angular_velocity * dt as f32 / 0.1;

                if position.position.x < -BOUNDARY {
                    boid.velocity.x *= -1.0;
                    position.position.x = position.position.x - (position.position.x + BOUNDARY);
                }
                if position.position.y < -BOUNDARY {
                    boid.velocity.y *= -1.0;
                    position.position.y = position.position.y - (position.position.y + BOUNDARY);
                }
                if position.position.z < -BOUNDARY {
                    boid.velocity.z *= -1.0;
                    position.position.z = position.position.z - (position.position.z + BOUNDARY);
                }
                if position.position.x > BOUNDARY {
                    boid.velocity.x *= -1.0;
                    position.position.x = position.position.x - (position.position.x - BOUNDARY);
                }
                if position.position.y > BOUNDARY {
                    boid.velocity.y *= -1.0;
                    position.position.y = position.position.y - (position.position.y - BOUNDARY);
                }
                if position.position.z > BOUNDARY {
                    boid.velocity.z *= -1.0;
                    position.position.z = position.position.z - (position.position.z - BOUNDARY);
                }
            }

            *time += dt;
        }

        let neuron_spheres: Vec<Sphere> = world
            .query::<(&Neuron, &Position)>()
            .iter()
            .map(|(_entity, (neuron, position))| {
                let value = ((neuron.dynamics[selected_boid].voltage + 100.0) / 150.0)
                    .clamp(0.0, 1.0) as f32;
                let color = match neuron.ty {
                    NeuronType::Excitatory => Vec3::new(value / 2.0, value, 0.95),
                    NeuronType::Inhibitory => Vec3::new(0.95, value / 2.0, value),
                };
                Sphere {
                    position: position.position,
                    color,
                    _padding: Default::default(),
                }
            })
            .collect();
        let boid_meshes: Vec<MeshInstanceData> = world
            .query::<(&Boid, &Position)>()
            .iter()
            .map(|(entity, (boid, position))| {
                let _color = if entity == *selected_boid {
                    Vec3::new(1.0, 0.3, 0.2)
                } else {
                    Vec3::new(1.0, 0.9, 1.0)
                };
                let rotation = Quat::from_rotation_arc(-Vec3::Z, boid.velocity.normalize());
                MeshInstanceData {
                    position: position.position,
                    rotation,
                    _padding: Default::default(),
                }
            })
            .collect();
        let trigger_spheres: Vec<Sphere> = world
            .query::<&Trigger>()
            .iter()
            .filter_map(|(_entity, trigger)| {
                if trigger.boid != *selected_boid {
                    return None;
                }
                let connection = world
                    .get::<&Connection>(trigger.connection)
                    .expect("Connection from broken");
                let start = world
                    .get::<&Position>(connection.from)
                    .expect("Connection from broken")
                    .position;
                let end = world
                    .get::<&Position>(connection.to)
                    .expect("Connection to broken")
                    .position;
                let diff = end - start;
                let position = start + diff * trigger.progress() as f32;
                Some(Sphere {
                    position,
                    color: Vec3::new(0.8, 0.9, 0.9),
                    _padding: Default::default(),
                })
            })
            .collect();

        let mut spheres = Vec::new();
        spheres.extend(neuron_spheres.iter());
        spheres.extend(trigger_spheres.iter());

        let mut bonds: Vec<BondData> = world
            .query::<&Connection>()
            .iter()
            .map(|(_, connection)| {
                let start = world
                    .get::<&Position>(connection.from)
                    .expect("Connection from broken")
                    .position;
                let end = world
                    .get::<&Position>(connection.to)
                    .expect("Connection to broken")
                    .position;
                BondData {
                    position_a: start,
                    position_b: end,
                    strength: connection.strength as f32,
                    _padding: Default::default(),
                }
            })
            .collect();

        if let Some(connection) = &connection_tool {
            bonds.push(BondData {
                position_a: connection.start,
                position_b: connection.end,
                strength: 1.0,
                _padding: Default::default(),
            });
        }

        self.sphere_buffer
            .update(&application.device, &application.queue, &spheres);

        self.line_buffer
            .update(&application.device, &application.queue, &bonds);

        self.mesh_instance_buffer
            .update(&application.device, &application.queue, &boid_meshes);
    }

    fn render(&mut self, data: &mut RenderData) {
        self.spheres.render(data);
        self.mesh.render(data);
        self.lines.render(data);
        self.boundaries.render(data);
    }

    fn gui(&mut self, _application: &visula::Application, context: &egui::Context) {
        egui::Window::new("Settings").show(context, |ui| {
            ui.label("Tool");
            for value in Tool::iter() {
                ui.selectable_value(&mut self.tool, value.clone(), format!("{:?}", &value));
            }
            ui.label("Simulation speed");
            ui.add(egui::Slider::new(&mut self.iterations, 1..=20));
        });
    }

    fn handle_event(&mut self, application: &mut visula::Application, event: &Event<CustomEvent>) {
        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state,
                        button: MouseButton::Left,
                        ..
                    },
                ..
            } => {
                self.mouse.left_down = *state == ElementState::Pressed;
                self.handle_tool(application);
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                self.mouse.position = Some(*position);
                self.handle_tool(application);
            }
            _ => {}
        }
    }
}

fn main() {
    visula::run(Simulation::new); //.expect("Initializing simulation failed"));
}
