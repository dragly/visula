use crate::InstanceDeviceExt;
use crate::RenderData;
use crate::{Application, InstanceBuffer, LineDelegate, Lines, Renderable};
use crate::{SphereDelegate, Spheres};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use visula_derive::Instance;

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Instance, Pod, Zeroable, Default)]
struct LineData {
    start: Vec3,
    end: Vec3,
    color: Vec3,
    _padding: [f32; 3],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Instance, Pod, Zeroable, Default)]
struct SphereData {
    position: Vec3,
    color: Vec3,
    radius: f32,
    _padding: f32,
}

#[derive(Clone, Copy, Debug, Instance, Default)]
pub struct Line {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Vec3,
}

#[derive(Clone, Copy, Debug, Instance, Default)]
pub struct Sphere {
    pub position: Vec3,
    pub color: Vec3,
    pub radius: f32,
}

pub struct Painter {
    line_data: Vec<LineData>,
    lines: Lines,
    line_buffer: InstanceBuffer<LineData>,
    sphere_data: Vec<SphereData>,
    spheres: Spheres,
    sphere_buffer: InstanceBuffer<SphereData>,
}

impl Painter {
    pub fn new(application: &mut Application) -> Self {
        let sphere_data = Vec::new();
        let sphere_buffer: InstanceBuffer<SphereData> = application.device.create_instance_buffer();
        let spheres_instance = sphere_buffer.instance();
        let spheres = Spheres::new(
            &application.rendering_descriptor(),
            &SphereDelegate {
                position: spheres_instance.position,
                radius: spheres_instance.radius,
                color: spheres_instance.color,
            },
        )
        .unwrap();

        let line_data = Vec::new();
        let line_buffer: InstanceBuffer<LineData> = application.device.create_instance_buffer();
        let line_instance = line_buffer.instance();
        let lines = Lines::new(
            &application.rendering_descriptor(),
            &LineDelegate {
                start: line_instance.start,
                end: line_instance.end,
                width: 2.0.into(),
                color: line_instance.color.clone(),
            },
        )
        .expect("Failed to create camera shape line");

        Self {
            line_data,
            line_buffer,
            lines,
            sphere_data,
            sphere_buffer,
            spheres,
        }
    }

    pub fn clear(&mut self) {
        self.line_data.clear();
        self.sphere_data.clear();
    }

    pub fn lines(&mut self, lines: &[Line]) {
        self.line_data.extend(lines.iter().map(|line| LineData {
            start: line.start,
            end: line.end,
            color: line.color,
            _padding: Default::default(),
        }));
    }
    pub fn spheres(&mut self, spheres: &[Sphere]) {
        self.sphere_data
            .extend(spheres.iter().map(|sphere| SphereData {
                position: sphere.position,
                color: sphere.color,
                radius: sphere.radius,
                _padding: Default::default(),
            }));
    }

    pub fn update(&self, application: &mut Application) {
        self.line_buffer
            .update(&application.device, &application.queue, &self.line_data);
        self.sphere_buffer
            .update(&application.device, &application.queue, &self.sphere_data);
    }

    pub fn render(&self, render_data: &mut RenderData) {
        self.lines.render(render_data);
        self.spheres.render(render_data);
    }
}
