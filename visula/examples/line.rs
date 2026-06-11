use bytemuck::{Pod, Zeroable};
use visula::{
    vec3, InstanceBuffer, InstanceDeviceExt, LineGeometry, LineMaterial, Lines, RenderData,
    Renderable,
};
use visula_derive::Instance;

#[repr(C)]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct LineData {
    position_a: [f32; 3],
    position_b: [f32; 3],
}

struct Simulation {
    lines: Lines,
    line_buffer: InstanceBuffer<LineData>,
    line_data: Vec<LineData>,
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Simulation {
        let line_buffer = application.device.create_instance_buffer::<LineData>();
        let line = line_buffer.instance();

        let lines = Lines::new(
            &application.rendering_descriptor(),
            &LineGeometry {
                start: line.position_a,
                end: line.position_b,
                width: 0.5.into(),
                color: vec3(0.8, 0.8, 0.8),
            },
            &LineMaterial::default(),
        )
        .unwrap();

        let line_data = vec![LineData {
            position_a: [-10.0, 0.0, 0.0],
            position_b: [10.0, 0.0, 0.0],
        }];

        Simulation {
            lines,
            line_buffer,
            line_data,
        }
    }
}

impl visula::Simulation for Simulation {
    fn update(&mut self, application: &mut visula::Application) {
        self.line_buffer
            .update(&application.device, &application.queue, &self.line_data);
    }

    fn render(&mut self, data: &mut RenderData) {
        self.lines.render(data);
    }
}

fn main() {
    visula::run(Simulation::new);
}
