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
pub mod camera;
pub mod custom_event;
pub mod drop_event;
pub mod error;
pub mod init_wgpu;
pub mod io;
pub mod pipelines;
pub mod primitives;
pub mod render_pass;
pub mod rendering_descriptor;
pub mod simulation;
pub mod vec_to_buffer;

#[cfg(not(target_arch = "wasm32"))]
pub mod setup_other;
#[cfg(target_arch = "wasm32")]
pub mod setup_wasm;

pub use application::Application;
pub use camera::controller::{CameraController, CameraControllerResponse};
pub use camera::Camera;
pub use custom_event::CustomEvent;
pub use drop_event::DropEvent;
pub use pipelines::*;
pub use primitives::*;
pub use render_pass::*;
pub use rendering_descriptor::RenderingDescriptor;
pub use simulation::*;

pub use visula_core::{
    glam, naga, uuid, wgpu, Expression, InstanceBuffer, InstanceDeviceExt, UniformBuffer,
};

pub use egui;
pub use web_sys;
pub use winit;
pub use wasm_bindgen;

pub type Vector2 = cgmath::Vector2<f32>;
pub type Vector3 = cgmath::Vector3<f32>;
pub type Matrix4 = cgmath::Matrix4<f32>;
pub type Point3 = cgmath::Point3<f32>;

pub struct RunConfig {
    pub canvas_name: String,
}

pub fn run<F, S>(init: F)
where
    F: FnMut(&mut Application) -> S + 'static,
    S: Simulation + 'static,
{
    run_with_config(
        RunConfig {
            canvas_name: "glcanvas".to_string(),
        },
        init,
    )
}

pub fn run_with_config<F, S>(config: RunConfig, mut init: F)
where
    F: FnMut(&mut Application) -> S + 'static,
    S: Simulation + 'static,
{
    let event_loop = EventLoopBuilder::<CustomEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("Visula");

    #[cfg(not(target_arch = "wasm32"))]
    println!(
        "NOTE: Ignoring canvas name on non-wasm platforms: '{}'",
        config.canvas_name
    );

    #[cfg(target_arch = "wasm32")]
    let mut canvas_existed = false;
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        if let Some(canvas) = document.get_element_by_id(&config.canvas_name) {
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
                simulation = Some(init(&mut app));
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
                        }
                    }
                }
            }
        }
    });
}
