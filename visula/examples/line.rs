use bytemuck::{Pod, Zeroable};

use visula::{
    initialize_event_loop_and_window, initialize_logger, Application, Expression, InstanceBuffer,
    LineDelegate, Lines, RenderData,
};
use visula_derive::Instance;
use wgpu::Color;
use winit::{event::Event, event_loop::ControlFlow};

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

async fn run() {
    initialize_logger();
    let (event_loop, window) = initialize_event_loop_and_window();
    let mut application = Application::new(window).await;

    let mut simulation = Simulation::new(&mut application).expect("Failed to init simulation");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::RedrawEventsCleared => {
                application.window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let frame = application.next_frame();
                let mut encoder = application.encoder();

                {
                    let view = application.begin_render_pass(&frame, &mut encoder, Color::BLACK);
                    simulation.render(&mut RenderData {
                        view: &view,
                        depth_texture: &application.depth_texture,
                        encoder: &mut encoder,
                        camera: &application.camera,
                    });
                }

                application.queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::MainEventsCleared => {
                application.update();
                simulation.update(&application);
            }
            event => {
                application.handle_event(&event, control_flow);
            }
        }
    });
}

fn main() {
    visula::spawn(run());
}
