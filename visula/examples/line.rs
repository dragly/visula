use bytemuck::{Pod, Zeroable};
use visula::{
    Expression, InstanceBuffer, InstanceDeviceExt, LineGeometry, LineMaterial, Lines, RenderData,
    Renderable,
};
use visula_derive::Instance;

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct LineData {
    position_a: [f32; 3],
    position_b: [f32; 3],
    _padding: [f32; 2],
}

#[derive(Debug)]
struct Error {}

struct Simulation {
    lines: Lines,
    line_buffer: InstanceBuffer<LineData>,
    line_data: Vec<LineData>,
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Result<Simulation, Error> {
        let line_buffer = application.device.create_instance_buffer::<LineData>();
        let line = line_buffer.instance();

        let lines = Lines::new(
            &application.rendering_descriptor(),
            &LineGeometry {
                start: line.position_a,
                end: line.position_b,
                width: 0.5.into(),
                color: Expression::Vector3 {
                    x: 0.8.into(),
                    y: 0.8.into(),
                    z: 0.8.into(),
                },
            },
            &LineMaterial {
                color: Expression::InputColor.lit(),
            },
        )
        .unwrap();

        let line_data = vec![LineData {
            position_a: [-10.0, 0.0, 0.0],
            position_b: [10.0, 0.0, 0.0],
            _padding: [0.0; 2],
        }];

        Ok(Simulation {
            lines,
            line_buffer,
            line_data,
        })
    }
}

impl visula::Simulation for Simulation {
    type Error = Error;

    fn update(&mut self, application: &mut visula::Application) {
        self.line_buffer
            .update(&application.device, &application.queue, &self.line_data);
    }

    fn render(&mut self, data: &mut RenderData) {
        self.lines.render(data);
    }
}

fn main() {
    visula::run(|app| Simulation::new(app).expect("Initializing simulation failed"));
}
