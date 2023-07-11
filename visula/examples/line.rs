use bytemuck::{Pod, Zeroable};

use visula::{Expression, InstanceBuffer, LineDelegate, Lines, RenderData};
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

impl visula::Simulation for Simulation {
    type Error = Error;
    fn init(application: &mut visula::Application) -> Result<Simulation, Error> {
        let line_buffer = InstanceBuffer::<LineData>::new(&application.device);
        let line = line_buffer.instance();

        let lines = Lines::new(
            &application.rendering_descriptor(),
            &LineDelegate {
                start: line.position_a,
                end: line.position_b,
                width: {
                    let a: Expression = 1.0.into();
                    a + 1.0 + 2.0
                },
                alpha: 1.0.into(),
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

    fn update(&mut self, application: &visula::Application) {
        self.line_buffer
            .update(&application.device, &application.queue, &self.line_data);
    }

    fn render(&mut self, data: &mut RenderData) {
        self.lines.render(data);
    }
}

fn main() {
    visula::run::<Simulation>();
}
