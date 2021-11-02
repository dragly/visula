use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

pub mod application;
pub mod bindings;
pub mod buffer;
pub mod camera;
pub mod custom_event;
pub mod drop_event;
pub mod error;
pub mod init_wgpu;
pub mod instances;
pub mod io;
pub mod naga_type;
pub mod pipelines;
pub mod primitives;
pub mod simulation;
pub mod vec_to_buffer;
pub mod vertex_attr;
pub mod vertex_attr_format;

pub use vertex_attr::VertexAttr;
pub use vertex_attr_format::VertexAttrFormat;

#[cfg(not(target_arch = "wasm32"))]
pub mod setup_other;
#[cfg(target_arch = "wasm32")]
pub mod setup_wasm;

pub use application::Application;
pub use bindings::*;
pub use buffer::Buffer;
pub use custom_event::CustomEvent;
pub use drop_event::DropEvent;
pub use instances::*;
pub use naga_type::NagaType;
pub use pipelines::*;
pub use primitives::*;
pub use simulation::Simulation;

pub type Vector2 = cgmath::Vector2<f32>;
pub type Vector3 = cgmath::Vector3<f32>;
pub type Matrix4 = cgmath::Matrix4<f32>;
pub type Point3 = cgmath::Point3<f32>;

pub fn run<S: 'static + Simulation>() {
    let event_loop = EventLoop::<CustomEvent>::with_user_event();
    let proxy = event_loop.create_proxy();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("Visula");
    let window = builder.build(&event_loop).unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    {
        setup_other::setup_other(window, proxy);
    }

    #[cfg(target_arch = "wasm32")]
    {
        // TODO should be enough with one proxy
        use std::cell::RefCell;
        use std::rc::Rc;
        let drop_proxy_main = Rc::new(RefCell::new(event_loop.create_proxy()));
        setup_wasm::setup_wasm(window, proxy, drop_proxy_main);
    }

    log::info!("Initializing application");

    let mut application: Option<Application> = None;
    let mut simulation: Option<S> = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::UserEvent(CustomEvent::Ready(mut app)) => {
                simulation = Some(S::init(&mut app).unwrap());
                application = Some(app);
            }
            event => {
                if let (Some(app), Some(sim)) = (&mut application, &mut simulation) {
                    match event {
                        Event::RedrawEventsCleared => {
                            app.window.request_redraw();
                        }
                        Event::RedrawRequested(_) => {
                            app.render(sim);
                        }
                        Event::MainEventsCleared => {
                            sim.update(app);
                        }
                        event => {
                            if let Event::WindowEvent {
                                event: ref window_event,
                                ..
                            } = event
                            {
                                sim.handle_event(app, window_event);
                            }
                            app.handle_event(&event, control_flow);
                        }
                    }
                }
            }
        }
    });
}
