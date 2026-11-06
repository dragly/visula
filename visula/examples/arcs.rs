use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use visula::{
    vec3, ArcGeometry, ArcMaterial, Arcs, InstanceBuffer, InstanceDeviceExt, RenderData, Renderable,
};
use visula_derive::Instance;

#[repr(C)]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct ArcData {
    start: [f32; 3],
    end: [f32; 3],
    turn: f32,
}

fn path(start: Vec3, heading: f32, steps: &[(f32, f32)]) -> Vec<ArcData> {
    let mut position = start;
    let mut heading = heading;
    steps
        .iter()
        .map(|&(length, turn)| {
            let chord = if turn == 0.0 {
                length
            } else {
                2.0 * length / turn.abs() * (0.5 * turn.abs()).sin()
            };
            let direction = heading + 0.5 * turn;
            let end = position + chord * Vec3::new(direction.cos(), direction.sin(), 0.0);
            let arc = ArcData {
                start: position.into(),
                end: end.into(),
                turn,
            };
            position = end;
            heading += turn;
            arc
        })
        .collect()
}

struct Simulation {
    arcs: Arcs,
    arc_buffer: InstanceBuffer<ArcData>,
    arc_data: Vec<ArcData>,
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Simulation {
        let camera = &mut application.camera_controller;
        camera.current_transform.forward = -Vec3::Z;
        camera.current_transform.distance = 36.0;
        camera.target_transform = camera.current_transform.clone();

        let arc_buffer = application.device.create_instance_buffer::<ArcData>();
        let arc = arc_buffer.instance();

        let arcs = Arcs::new(
            &application.rendering_descriptor(),
            &ArcGeometry {
                start: arc.start,
                end: arc.end,
                turn: arc.turn,
                width: 0.6.into(),
                color: vec3(0.8, 0.8, 0.8),
                ..Default::default()
            },
            &ArcMaterial::default(),
        )
        .unwrap();

        let metro = path(
            Vec3::new(-12.0, -6.0, 0.0),
            0.0,
            &[
                (4.0, 0.0),
                (1.5, FRAC_PI_4),
                (4.0, 0.0),
                (1.5, -FRAC_PI_4),
                (3.0, 0.0),
                (2.5, FRAC_PI_2),
                (3.0, 0.0),
            ],
        );
        let wave = path(
            Vec3::new(-12.0, 4.0, 0.0),
            0.0,
            &[(3.0, PI), (3.0, -PI), (3.0, PI), (2.0, 0.0)],
        );
        let ring = path(Vec3::new(8.0, 6.0, 0.0), 0.0, &[(14.0, 1.5 * PI)]);

        Simulation {
            arcs,
            arc_buffer,
            arc_data: [metro, wave, ring].concat(),
        }
    }
}

impl visula::Simulation for Simulation {
    fn update(&mut self, application: &mut visula::Application) {
        self.arc_buffer
            .update(&application.device, &application.queue, &self.arc_data);
    }

    fn render(&mut self, data: &mut RenderData) {
        self.arcs.render(data);
    }
}

fn main() {
    visula::run(Simulation::new);
}
