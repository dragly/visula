use wgpu::TextureViewDescriptor;

use bytemuck::{Pod, Zeroable};
use visula::{create_application, create_window, CustomEvent, Renderable};
use visula::{
    initialize_logger, Application, Expression, InstanceBuffer, LineDelegate, Lines, RenderData,
};
use visula_derive::Instance;
use wgpu::Color;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopProxy;

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
                color: Expression::Vector3 {
                    x: 1.0.into(),
                    y: 1.0.into(),
                    z: 1.0.into(),
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

struct App {
    application: Option<Application>,
    simulation: Option<Simulation>,
    event_loop_proxy: EventLoopProxy<CustomEvent>,
}

impl ApplicationHandler<CustomEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = create_window(event_loop);
        create_application(window, &self.event_loop_proxy);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(ref mut application) = self.application else {
            return;
        };
        if self.simulation.is_none() {
            let simulation = Simulation::new(application).expect("Failed to init simulation");
            self.simulation = Some(simulation);
        };
        let Some(ref mut simulation) = self.simulation else {
            panic!("Simulation must be set at this point!");
        };
        match event {
            WindowEvent::RedrawRequested => {
                application.update();
                simulation.update(application);
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
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: CustomEvent) {
        match event {
            CustomEvent::Application(application) => {
                application.window.request_redraw();
                self.application = Some(*application);
            }
            CustomEvent::DropEvent(_) => {}
        }
    }
}

fn main() -> Result<(), EventLoopError> {
    initialize_logger();
    let event_loop = winit::event_loop::EventLoop::with_user_event().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App {
        application: None,
        simulation: None,
        event_loop_proxy: event_loop.create_proxy(),
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
