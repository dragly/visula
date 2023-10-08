#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;
#[cfg(target_arch = "wasm32")]
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowBuilderExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use winit::window::Window;

use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};

pub mod application;
pub mod camera;
pub mod custom_event;
pub mod drop_event;
pub mod error;
pub mod io;
pub mod pipelines;
pub mod primitives;
pub mod render_pass;
pub mod rendering_descriptor;
pub mod simulation;
pub mod vec_to_buffer;

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
pub use wasm_bindgen;
pub use web_sys;
pub use winit;

pub type Vector2 = cgmath::Vector2<f32>;
pub type Vector3 = cgmath::Vector3<f32>;
pub type Matrix3 = cgmath::Matrix3<f32>;
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

pub async fn start<F, S>(event_loop: EventLoop<CustomEvent>, window: Window, mut init_simulation: F)
where
    F: FnMut(&mut Application) -> S + 'static,
    S: Simulation + 'static,
{
    let mut application = Application::new(window).await;

    let mut simulation: S = init_simulation(&mut application);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::RedrawEventsCleared => {
                application.window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                application.render(&mut simulation);
            }
            Event::MainEventsCleared => {
                application.update();
                simulation.update(&application);
            }
            event => {
                if !application.handle_event(&event, control_flow) {
                    simulation.handle_event(&mut application, &event);
                }
            }
        }
    });
}

pub fn initialize_panic_hook() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    }
}

pub fn initialize_logger() {
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init().expect("could not initialize logger");
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
}

pub fn initialize_event_loop_and_window(_config: RunConfig) -> (EventLoop<CustomEvent>, Window) {
    let event_loop = EventLoopBuilder::<CustomEvent>::with_user_event().build();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("Visula");

    #[cfg(not(target_arch = "wasm32"))]
    log::info!(
        "Ignoring canvas name on non-wasm platforms: '{}'",
        _config.canvas_name
    );

    #[cfg(target_arch = "wasm32")]
    let mut canvas_existed = false;
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        if let Some(canvas) = document.get_element_by_id(&_config.canvas_name) {
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

    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        let drop_proxy_main = Rc::new(RefCell::new(event_loop.create_proxy()));
        log::info!("Start");
        use winit::platform::web::WindowExtWebSys;
        let drag_enter = Closure::wrap(Box::new(|event: &web_sys::Event| {
            event.prevent_default();
            log::info!("Drag enter!");
        }) as Box<dyn FnMut(&web_sys::Event)>);
        let drag_over = Closure::wrap(Box::new(|event: &web_sys::Event| {
            event.prevent_default();
            log::info!("Drag over!");
        }) as Box<dyn FnMut(&web_sys::Event)>);

        let drop_callback = Closure::wrap(Box::new(move |event: &web_sys::Event| {
            event.prevent_default();
            let drag_event_ref: &web_sys::DragEvent = JsCast::unchecked_from_js_ref(event);
            let drag_event = drag_event_ref.clone();
            match drag_event.data_transfer() {
                None => {}
                Some(data_transfer) => match data_transfer.files() {
                    None => {}
                    Some(files) => {
                        log::info!("Files {:?}", files.length());
                        for i in 0..files.length() {
                            if let Some(file) = files.item(i) {
                                log::info!("Processing file {i}");
                                let drop_proxy_ref = Rc::clone(&drop_proxy_main);
                                let name = file.name();
                                let read_callback =
                                    Closure::wrap(Box::new(move |array_buffer: JsValue| {
                                        let array = Uint8Array::new(&array_buffer);
                                        let bytes: Vec<u8> = array.to_vec();
                                        let event_result = (*drop_proxy_ref)
                                            .borrow_mut()
                                            .send_event(CustomEvent::DropEvent(DropEvent {
                                                name: name.clone(),
                                                bytes,
                                            }));
                                        log::info!("Sent event");
                                        match event_result {
                                            Ok(_) => {}
                                            Err(_) => {
                                                log::error!(
                                            "Could not register drop event! Event loop closed?"
                                        );
                                            }
                                        }
                                    })
                                        as Box<dyn FnMut(JsValue)>);
                                let _ = file.array_buffer().then(&read_callback);
                                read_callback.forget();
                            }
                        }
                    }
                },
            }
        }) as Box<dyn FnMut(&web_sys::Event)>);

        log::info!("Setting up drag and drop features");
        web_sys::window()
            .and_then(|win| {
                win.set_ondragenter(Some(JsCast::unchecked_from_js_ref(drag_enter.as_ref())));
                win.set_ondragover(Some(JsCast::unchecked_from_js_ref(drag_over.as_ref())));
                win.set_ondrop(Some(JsCast::unchecked_from_js_ref(drop_callback.as_ref())));
                win.document()
            })
            .expect("could not set up window");

        // From the rustwasm documentation:
        //
        // The instance of `Closure` that we created will invalidate its
        // corresponding JS callback whenever it is dropped, so if we were to
        // normally return from `main` then our registered closure will
        // raise an exception when invoked.
        //
        // Normally we'd store the handle to later get dropped at an appropriate
        // time but for now we want it to be a global handler so we use the
        // `forget` method to drop it without invalidating the closure. Note that
        // this is leaking memory in Rust, so this should be done judiciously!
        drag_enter.forget();
        drag_over.forget();
        drop_callback.forget();
    }

    (event_loop, window)
}

pub fn run_with_config<F, S>(_config: RunConfig, init: F)
where
    F: FnMut(&mut Application) -> S + 'static,
    S: Simulation + 'static,
{
    initialize_logger();
    let (event_loop, window) = initialize_event_loop_and_window(_config);

    log::info!("Initializing application");

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(start(event_loop, window, init))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(start(event_loop, window, init))
    }
}
