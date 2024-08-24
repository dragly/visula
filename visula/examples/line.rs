use std::sync::Arc;
use wgpu::TextureViewDescriptor;

use bytemuck::{Pod, Zeroable};
use visula::Renderable;
use visula::{
    initialize_event_loop_and_window, initialize_logger, Application, Expression, InstanceBuffer,
    LineDelegate, Lines, RenderData,
};
use visula_derive::Instance;
use wgpu::Color;
use winit::event::Event;
use winit::event::WindowEvent;

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
                start_color: Expression::Vector3 {
                    x: 1.0.into(),
                    y: 1.0.into(),
                    z: 1.0.into(),
                },
                end_color: Expression::Vector3 {
                    x: 0.0.into(),
                    y: 0.0.into(),
                    z: 0.0.into(),
                },
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

    fn update(&mut self, application: &mut visula::Application) {
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
    let mut application = Application::new(Arc::new(window)).await;

    let mut simulation = Simulation::new(&mut application).expect("Failed to init simulation");

    event_loop
        .run(move |event, target| match event {
            Event::WindowEvent {
                window_id: _,
                event,
            } => match event {
                WindowEvent::RedrawRequested => {
                    application.update();
                    simulation.update(&mut application);
                    let frame = application.next_frame();
                    let mut encoder = application.encoder();

                    {
                        let view = frame.texture.create_view(&TextureViewDescriptor {
                            format: Some(application.config.view_formats[0]),
                            ..wgpu::TextureViewDescriptor::default()
                        });
                        application.clear(&view, &mut encoder, Color::BLACK);
                        simulation.render(&mut RenderData {
                            view: &view,
                            multisampled_framebuffer: &application.multisampled_framebuffer,
                            depth_texture: &application.depth_texture,
                            encoder: &mut encoder,
                            camera: &application.camera,
                        });
                    }

                    application.queue.submit(Some(encoder.finish()));
                    frame.present();
                    application.window.request_redraw();
                }
                WindowEvent::CloseRequested => target.exit(),
                _ => {}
            },
            event => {
                application.handle_event(&event);
            }
        })
        .expect("Failed to run event loop");
}

fn main() {
    visula::spawn(run());
}
