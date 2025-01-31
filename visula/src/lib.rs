#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;
use winit::application::ApplicationHandler;
#[cfg(target_arch = "wasm32")]
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::EventLoop;
use winit::event_loop::EventLoopProxy;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowAttributesExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::window::WindowId;

use std::borrow::BorrowMut;
use std::future::Future;
use std::sync::Arc;
use winit::window::Window;

use winit::{event::Event, event_loop::EventLoopBuilder};

pub mod derive {
    pub use visula_derive::*;
}
pub use bytemuck;
pub use cgmath;
pub use visula_core;
pub use visula_core::{
    glam, naga, uuid, wgpu, Expression, InstanceBuffer, InstanceDeviceExt, UniformBuffer,
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

struct App<F, S> {
    application: Option<Application>,
    simulation: Option<S>,
    event_loop_proxy: EventLoopProxy<CustomEvent>,
    main_window_id: Option<WindowId>,
    init_simulation: F,
}

impl<F, S> ApplicationHandler<CustomEvent> for App<F, S>
where
    F: FnMut(&mut Application) -> S + 'static,
    S: Simulation + 'static,
{
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = create_window(event_loop);
        self.main_window_id = Some(window.id());
        create_application(window, &self.event_loop_proxy);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(ref mut application) = self.application else {
            return;
        };
        if self.simulation.is_none() {
            let simulation = (self.init_simulation)(application);
            self.simulation = Some(simulation);
        };
        let Some(ref mut simulation) = self.simulation else {
            panic!("Simulation must be set at this point!");
        };
        if !application.window_event(window_id, &event) {
            simulation.handle_event(
                application,
                &Event::<CustomEvent>::WindowEvent {
                    window_id,
                    event: event.clone(),
                },
            );
        }
        if self.main_window_id.unwrap() != window_id {
            return;
        }
        match event {
            WindowEvent::RedrawRequested => {
                application.update();
                simulation.update(application);
                application.render(simulation);
                application.window.borrow_mut().request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: CustomEvent) {
        match event {
            CustomEvent::Application(application) => self.application = Some(application),
            CustomEvent::DropEvent(_) => {}
        }
    }
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

// pub async fn start<F, S>(
//     event_loop: EventLoop<CustomEvent>,
//     window: WindowState,
//     mut init_simulation: F,
// ) where
//     F: FnMut(&mut Application) -> S + 'static,
//     S: Simulation + 'static,
// {
//     let main_window_id = window.id();
//     let mut application = Application::new(Arc::new(window)).await;
//
//     let mut simulation: S = init_simulation(&mut application);
//
//     event_loop
//         .run(move |event, target| {
//             if !application.handle_event(&event) {
//                 simulation.handle_event(&mut application, &event);
//             }
//             if let Event::WindowEvent { window_id, event } = event {
//                 if main_window_id != window_id {
//                     return;
//                 }
//                 match event {
//                     WindowEvent::RedrawRequested => {
//                         application.update();
//                         simulation.update(&mut application);
//                         application.render(&mut simulation);
//                         application.window.borrow_mut().request_redraw();
//                     }
//                     WindowEvent::CloseRequested => target.exit(),
//                     _ => {}
//                 }
//             }
//         })
//         .expect("Event loop failed to run");
// }

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

pub fn initialize_event_loop_and_window() -> (EventLoop<CustomEvent>, WindowState) {
    initialize_event_loop_and_window_with_config(RunConfig {
        canvas_name: "glcanvas".to_string(),
    })
}

pub fn create_event_loop() -> EventLoop<CustomEvent> {
    EventLoop::<CustomEvent>::with_user_event()
        .build()
        .expect("Failed to create event loop")
}

pub fn create_window_with_config(config: RunConfig, event_loop: &ActiveEventLoop) -> Arc<Window> {
    let mut builder = winit::window::Window::default_attributes();
    builder = builder.with_title("Visula");

    #[cfg(not(target_arch = "wasm32"))]
    {
        log::info!(
            "Ignoring canvas name on non-wasm platforms: '{}'",
            config.canvas_name
        );
        Arc::new(event_loop.create_window(builder).unwrap())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut canvas_existed = false;
        let web_window = web_sys::window().expect("no global `window` exists");
        let document = web_window
            .document()
            .expect("should have a document on window");
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
        let window = event_loop.create_window(builder).unwrap();
        if !canvas_existed {
            let window_canvas = window.canvas().expect("should have made canvas");
            window_canvas
                .set_attribute("style", "width: 100%; height: 100%")
                .unwrap();
            web_sys::window()
                .expect("no global `window` exists")
                .document()
                .expect("should have a document on window")
                .body()
                .expect("should have a body on document")
                .append_child(&web_sys::Element::from(window_canvas))
                .ok()
                .expect("couldn't append canvas to document body");
        }
        Arc::new(window)
    }
}

pub fn create_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    create_window_with_config(
        RunConfig {
            canvas_name: "canvas".to_owned(),
        },
        event_loop,
    )
}

// use std::cell::RefCell;
// use std::rc::Rc;
// let drop_proxy_main = Rc::new(RefCell::new(event_loop.create_proxy()));
// log::info!("Start");
// use winit::platform::web::WindowExtWebSys;
// let drag_enter = Closure::wrap(Box::new(|event: &web_sys::Event| {
//     event.prevent_default();
//     log::info!("Drag enter!");
// }) as Box<dyn FnMut(&web_sys::Event)>);
// let drag_over = Closure::wrap(Box::new(|event: &web_sys::Event| {
//     event.prevent_default();
//     log::info!("Drag over!");
// }) as Box<dyn FnMut(&web_sys::Event)>);
//
// let drop_callback = Closure::wrap(Box::new(move |event: &web_sys::Event| {
//     event.prevent_default();
//     let drag_event_ref: &web_sys::DragEvent = JsCast::unchecked_from_js_ref(event);
//     let drag_event = drag_event_ref.clone();
//     match drag_event.data_transfer() {
//         None => {}
//         Some(data_transfer) => match data_transfer.files() {
//             None => {}
//             Some(files) => {
//                 log::info!("Files {:?}", files.length());
//                 for i in 0..files.length() {
//                     if let Some(file) = files.item(i) {
//                         log::info!("Processing file {i}");
//                         let drop_proxy_ref = Rc::clone(&drop_proxy_main);
//                         let name = file.name();
//                         let read_callback =
//                             Closure::wrap(Box::new(move |array_buffer: JsValue| {
//                                 let array = Uint8Array::new(&array_buffer);
//                                 let bytes: Vec<u8> = array.to_vec();
//                                 let event_result = (*drop_proxy_ref)
//                                     .borrow_mut()
//                                     .send_event(CustomEvent::DropEvent(DropEvent {
//                                         name: name.clone(),
//                                         bytes,
//                                     }));
//                                 log::info!("Sent event");
//                                 match event_result {
//                                     Ok(_) => {}
//                                     Err(_) => {
//                                         log::error!(
//                                     "Could not register drop event! Event loop closed?"
//                                 );
//                                     }
//                                 }
//                             })
//                                 as Box<dyn FnMut(JsValue)>);
//                         let _ = file.array_buffer().then(&read_callback);
//                         read_callback.forget();
//                     }
//                 }
//             }
//         },
//     }
// }) as Box<dyn FnMut(&web_sys::Event)>);
//
// log::info!("Setting up drag and drop features");
// web_sys::window()
//     .and_then(|win| {
//         win.set_ondragenter(Some(JsCast::unchecked_from_js_ref(drag_enter.as_ref())));
//         win.set_ondragover(Some(JsCast::unchecked_from_js_ref(drag_over.as_ref())));
//         win.set_ondrop(Some(JsCast::unchecked_from_js_ref(drop_callback.as_ref())));
//         win.document()
//     })
//     .expect("could not set up window");
//
// // From the rustwasm documentation:
// //
// // The instance of `Closure` that we created will invalidate its
// // corresponding JS callback whenever it is dropped, so if we were to
// // normally return from `main` then our registered closure will
// // raise an exception when invoked.
// //
// // Normally we'd store the handle to later get dropped at an appropriate
// // time but for now we want it to be a global handler so we use the
// // `forget` method to drop it without invalidating the closure. Note that
// // this is leaking memory in Rust, so this should be done judiciously!
// drag_enter.forget();
// drag_over.forget();
// drop_callback.forget();

struct WindowState {
    config: RunConfig,
    window: Option<Window>,
}

pub fn initialize_event_loop_and_window_with_config(
    config: RunConfig,
) -> (EventLoop<CustomEvent>, WindowState) {
    let event_loop = create_event_loop();
    let window_state = WindowState {
        config,
        window: None,
    };

    (event_loop, window_state)
}

pub fn create_application(window: Arc<Window>, event_loop_proxy: &EventLoopProxy<CustomEvent>) {
    let proxy = event_loop_proxy.clone();
    let window_handle = window.clone();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let application = pollster::block_on(async move { Application::new(window_handle).await });
        assert!(proxy
            .send_event(CustomEvent::Application(application))
            .is_ok());
    }

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            let application = Application::new(window_handle).await;
            assert!(proxy
                .send_event(CustomEvent::Application(application))
                .is_ok());
        });
    }
}

pub fn spawn<F>(f: F)
where
    F: Future<Output = ()> + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(f);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(f);
    }
}

pub fn run_with_config<F, S>(config: RunConfig, init: F)
where
    F: FnMut(&mut Application) -> S + 'static,
    S: Simulation + 'static,
{
    initialize_logger();
    initialize_panic_hook();
    let event_loop = create_event_loop();
    let mut app = App {
        application: None,
        simulation: None,
        init_simulation: init,
        event_loop_proxy: event_loop.create_proxy(),
        main_window_id: None,
    };

    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop")
}
