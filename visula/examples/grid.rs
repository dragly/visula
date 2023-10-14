use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, SquareMatrix};
use glam::Vec3;
use itertools::iproduct;
use visula::{
    CustomEvent, Expression, InstanceBuffer, LineDelegate, Lines, RenderData, Renderable,
    UniformBuffer,
};
use visula_derive::{Instance, Uniform};
use winit::event::{Event, WindowEvent};

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
    uniforms_buffer: UniformBuffer<Uniforms>,
    uniforms_data: Uniforms,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Uniform, Pod, Zeroable)]
struct Uniforms {
    cursor_position: Vec3,
    _padding: f32,
}

fn gaussian(a: &Expression, b: &Expression) -> Expression {
    Expression::Vector3 {
        x: 0.0.into(),
        y: (10.0 * &(-((a - b).length()).pow(2.0) / (2.0 * 8.0)).exp()).into(),
        z: 0.0.into(),
    }
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Result<Simulation, Error> {
        let column_count = 100;
        let row_count = 100;
        let columns = 0..column_count;
        let rows = 0..row_count;

        let directions = [[1.0, 0.0], [0.0, 1.0]];
        let offset = [-column_count as f32 / 2.0, -row_count as f32 / 2.0];

        let line_data: Vec<LineData> = iproduct!(directions, columns, rows)
            .map(|(direction, column, row)| LineData {
                position_a: [offset[0] + column as f32, 0.0, offset[1] + row as f32],
                position_b: [
                    offset[0] + column as f32 + direction[0],
                    0.0,
                    offset[1] + row as f32 + direction[1],
                ],
                _padding: [0.0, 0.0],
            })
            .collect();

        let uniforms_data = Uniforms {
            cursor_position: Vec3::new(0.0, 0.0, 0.0),
            _padding: 0.0,
        };
        let uniforms_buffer =
            UniformBuffer::<Uniforms>::new_with_init(&application.device, &uniforms_data);
        let uniforms = uniforms_buffer.uniform();

        let line_buffer = InstanceBuffer::<LineData>::new(&application.device);
        let line = line_buffer.instance();
        let offset_a = gaussian(&line.position_a, &uniforms.cursor_position);
        let offset_b = gaussian(&line.position_b, &uniforms.cursor_position);
        let lines = Lines::new(
            &application.rendering_descriptor(),
            &LineDelegate {
                start: &line.position_a + &offset_a,
                end: line.position_b + &offset_b,
                width: 0.1.into(),
                ..Default::default()
            },
        )
        .unwrap();

        Ok(Simulation {
            lines,
            line_buffer,
            line_data,
            uniforms_data,
            uniforms_buffer,
        })
    }
}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn update(&mut self, application: &visula::Application) {
        self.line_buffer
            .update(&application.device, &application.queue, &self.line_data);
    }

    fn render(&mut self, data: &mut RenderData) {
        self.lines.render(data);
    }

    fn handle_event(&mut self, application: &mut visula::Application, event: &Event<CustomEvent>) {
        let Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } = event
        else {
            return;
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
        let aspect_ratio = application.config.width as f32 / application.config.height as f32;
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
        self.uniforms_data.cursor_position = intersection;
        self.uniforms_buffer
            .update(&application.queue, &self.uniforms_data);
    }
}

fn main() {
    visula::run(|app| Simulation::new(app).expect("Initializing simulation failed"));
}
