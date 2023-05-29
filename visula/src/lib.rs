#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;
#[cfg(target_arch = "wasm32")]
use winit::dpi::LogicalSize;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowBuilderExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};

pub mod application;
pub mod bindings;
pub mod camera;
pub mod custom_event;
pub mod drop_event;
pub mod error;
pub mod init_wgpu;
pub mod instance_buffer;
pub mod instances;
pub mod io;
pub mod naga_type;
pub mod pipelines;
pub mod primitives;
pub mod render_pass;
pub mod rendering_descriptor;
pub mod simulation;
pub mod uniform_buffer;
pub mod value;
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
pub use custom_event::CustomEvent;
pub use drop_event::DropEvent;
pub use instance_buffer::*;
pub use instances::*;
pub use naga_type::*;
pub use pipelines::*;
pub use primitives::*;
pub use render_pass::*;
pub use simulation::*;
pub use uniform_buffer::*;
pub use value::*;

pub type Vector2 = cgmath::Vector2<f32>;
pub type Vector3 = cgmath::Vector3<f32>;
pub type Matrix4 = cgmath::Matrix4<f32>;
pub type Point3 = cgmath::Point3<f32>;

pub fn run<S: 'static + Simulation>() {
    let event_loop = EventLoopBuilder::<CustomEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("Visula");

    #[cfg(target_arch = "wasm32")]
    let mut canvas_existed = false;
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        if let Some(canvas) = document.get_element_by_id("glcanvas") {
            let canvas = canvas
                .dyn_into::<HtmlCanvasElement>()
                .expect("could not cast to HtmlCanvasElement");

            builder = builder.with_inner_size(LogicalSize::new(
                canvas.client_width(),
                canvas.client_height(),
            ));
            builder = builder.with_canvas(Some(canvas));
            canvas_existed = true;
        }
    }
    let window = builder.build(&event_loop).unwrap();
    #[cfg(target_arch = "wasm32")]
    {
        if !canvas_existed {
            web_sys::window()
                .expect("no global `window` exists")
                .document()
                .expect("should have a document on window")
                .body()
                .expect("should have a body on document")
                .append_child(&web_sys::Element::from(window.canvas()))
                .ok()
                .expect("couldn't append canvas to document body");
        }
    }

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
                application = Some(*app);
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
                            app.update();
                            sim.update(app);
                        }
                        event => {
                            if !app.handle_event(&event, control_flow) {
                                sim.handle_event(app, &event);
                            }
                            app.handle_event(&event, control_flow);
                        }
                    }
                }
            }
        }
    });
}
