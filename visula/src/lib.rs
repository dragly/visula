#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
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
use winit::platform::web::WindowAttributesExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::window::WindowId;

use std::borrow::BorrowMut;
use std::future::Future;
use std::sync::Arc;
use winit::window::Window;

use winit::event::Event;

pub mod derive {
    pub use visula_derive::*;
}
pub use bytemuck;
pub use cgmath;
pub use egui_wgpu;
pub use visula_core;
pub use visula_core::{
    glam, naga, uuid, wgpu, Expression, InstanceBuffer, InstanceDeviceExt, TextureInput,
    UniformBuffer,
};

pub mod application;
pub mod camera;
pub mod custom_event;
pub mod drop_event;
pub mod error;
pub mod io;
pub mod light;
pub mod painter;
pub mod pipelines;
pub mod post_process;
pub mod primitives;
pub mod render_pass;
pub mod rendering_controls;
pub mod rendering_descriptor;
pub mod simulation;
pub mod text;
pub mod vec_to_buffer;

pub use application::Application;
pub use camera::controller::{CameraController, CameraControllerResponse, CameraTransform};
pub use camera::Camera;
pub use custom_event::CustomEvent;
pub use drop_event::DropEvent;
pub use light::DirectionalLight;
pub use pipelines::*;
pub use primitives::*;
pub use render_pass::*;
pub use rendering_controls::RenderingControls;
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
    config: RunConfig,
    application: Option<Application>,
    simulation: Option<S>,
    event_loop_proxy: EventLoopProxy<CustomEvent>,
    main_window_id: Option<WindowId>,
    init_simulation: F,
}

impl<F, S, E> ApplicationHandler<CustomEvent> for App<F, S>
where
    F: FnMut(&mut Application) -> Result<S, E> + 'static,
    S: Simulation + 'static,
    E: std::fmt::Debug + 'static,
{
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window =
            create_window_with_config(&self.config, event_loop).expect("Failed to create window");
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
            let simulation =
                (self.init_simulation)(application).expect("Failed to initialize simulation");
            self.simulation = Some(simulation);
        };
        let Some(ref mut simulation) = self.simulation else {
            panic!("Simulation must be set at this point");
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
        if self.main_window_id.is_none_or(|id| id != window_id) {
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

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let Some(ref mut application) = self.application else {
            return;
        };
        application.device_event(event_loop, device_id, &event);
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

pub fn run<F, S, E>(init: F) -> Result<(), error::Error>
where
    F: FnMut(&mut Application) -> Result<S, E> + 'static,
    S: Simulation + 'static,
    E: std::fmt::Debug + 'static,
{
    run_with_config(
        RunConfig {
            canvas_name: "glcanvas".to_string(),
        },
        init,
    )
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
        let _ = console_log::init();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
}

pub fn create_event_loop() -> Result<EventLoop<CustomEvent>, error::Error> {
    Ok(EventLoop::<CustomEvent>::with_user_event().build()?)
}

pub fn create_window_with_config(
    config: &RunConfig,
    event_loop: &ActiveEventLoop,
) -> Result<Arc<Window>, error::Error> {
    let mut builder = winit::window::Window::default_attributes();
    builder = builder.with_title("Visula");

    #[cfg(not(target_arch = "wasm32"))]
    {
        log::info!(
            "Ignoring canvas name on non-wasm platforms: '{}'",
            config.canvas_name
        );
        Ok(Arc::new(event_loop.create_window(builder)?))
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut canvas_existed = false;
        let web_window = web_sys::window().ok_or(error::Error::WebWindow)?;
        let document = web_window.document().ok_or(error::Error::WebDocument)?;
        if let Some(canvas) = document.get_element_by_id(&config.canvas_name) {
            let canvas = canvas.dyn_into::<HtmlCanvasElement>().map_err(|_| {
                error::Error::WebCanvas("could not cast to HtmlCanvasElement".into())
            })?;

            builder = builder.with_inner_size(LogicalSize::new(
                canvas.client_width(),
                canvas.client_height(),
            ));
            builder = builder.with_canvas(Some(canvas));
            canvas_existed = true;
        }
        builder = builder.with_active(false);
        let window = event_loop.create_window(builder)?;
        if !canvas_existed {
            let window_canvas = window
                .canvas()
                .ok_or(error::Error::WebCanvas("window has no canvas".into()))?;
            window_canvas
                .set_attribute("style", "width: 100%; height: 100%")
                .map_err(|_| error::Error::WebCanvas("could not set canvas style".into()))?;
            let body = web_sys::window()
                .ok_or(error::Error::WebWindow)?
                .document()
                .ok_or(error::Error::WebDocument)?
                .body()
                .ok_or(error::Error::WebDom("no body on document".into()))?;
            body.append_child(&web_sys::Element::from(window_canvas))
                .map_err(|_| error::Error::WebDom("could not append canvas to body".into()))?;
        }
        Ok(Arc::new(window))
    }
}

pub fn create_window(event_loop: &ActiveEventLoop) -> Result<Arc<Window>, error::Error> {
    create_window_with_config(
        &RunConfig {
            canvas_name: "canvas".to_owned(),
        },
        event_loop,
    )
}

pub fn create_application(window: Arc<Window>, event_loop_proxy: &EventLoopProxy<CustomEvent>) {
    let proxy = event_loop_proxy.clone();
    let window_handle = window.clone();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let application = pollster::block_on(async move { Application::new(window_handle).await })
            .expect("Failed to create application");
        let _ = proxy.send_event(CustomEvent::Application(Box::new(application)));
    }

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            let application = Application::new(window_handle)
                .await
                .expect("Failed to create application");
            let _ = proxy.send_event(CustomEvent::Application(Box::new(application)));
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

pub fn run_with_config<F, S, E>(config: RunConfig, init: F) -> Result<(), error::Error>
where
    F: FnMut(&mut Application) -> Result<S, E> + 'static,
    S: Simulation + 'static,
    E: std::fmt::Debug + 'static,
{
    initialize_logger();
    initialize_panic_hook();
    let event_loop = create_event_loop()?;
    let mut app = App {
        config,
        application: None,
        simulation: None,
        init_simulation: init,
        event_loop_proxy: event_loop.create_proxy(),
        main_window_id: None,
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}
