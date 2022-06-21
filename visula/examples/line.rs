use bytemuck::{Pod, Zeroable};
use wgpu::BufferUsages;

use visula::{
    BindingBuilder, Buffer, BufferBinding, BufferBindingField, BufferInner, Instance,
    InstanceField, InstanceHandle, LineDelegate, Lines, NagaType, SimulationRenderData,
    VertexAttrFormat, VertexBufferLayoutBuilder,
};
use visula_derive::{delegate, Instance};

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
    line_buffer: Buffer<LineData>,
    line_data: Vec<LineData>,
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn init(application: &mut visula::Application) -> Result<Simulation, Error> {
        let line_buffer = Buffer::<LineData>::new(
            application,
            BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::COPY_DST,
            "line",
        );
        let line = line_buffer.instance();

        let lines = Lines::new(
            application,
            &LineDelegate {
                start: delegate!(line.position_a),
                end: delegate!(line.position_b),
                width: delegate!(1.0),
                alpha: delegate!(1.0),
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
        self.line_buffer.update(application, &self.line_data);
    }

    fn render(&mut self, data: &mut SimulationRenderData) {
        self.lines.render(data);
    }
}

fn main() {
    visula::run::<Simulation>();
}
