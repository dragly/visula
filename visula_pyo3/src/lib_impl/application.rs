use egui::Slider;

use std::path::PathBuf;
use std::time::Duration;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopProxy;

use pyo3::prelude::*;
use pyo3::types::PyFunction;

use visula::{
    create_application, create_window, Application, CustomEvent, RenderData, Renderable, Simulation,
};

use winit::platform::pump_events::EventLoopExtPumpEvents;

use super::{PyEventLoop, PySlider};

struct AutoScreenshot {
    path: PathBuf,
    capture_after_frames: u32,
    frames_rendered: u32,
    captured: bool,
}

impl AutoScreenshot {
    fn from_env() -> Option<Self> {
        let path = std::env::var_os("VISULA_SCREENSHOT")?;
        let capture_after_frames = std::env::var("VISULA_SCREENSHOT_FRAMES")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(30);
        Some(AutoScreenshot {
            path: PathBuf::from(path),
            capture_after_frames,
            frames_rendered: 0,
            captured: false,
        })
    }
}

struct PySimulation<'a> {
    renderables: &'a [Box<dyn Renderable>],
    controls: &'a mut Vec<Py<PySlider>>,
}

impl Simulation for PySimulation<'_> {
    fn render(&mut self, data: &mut RenderData) {
        for renderable in self.renderables {
            renderable.render(data);
        }
    }

    fn render_shadow(&mut self, data: &mut visula::ShadowRenderData) {
        for renderable in self.renderables {
            renderable.render_shadow(data);
        }
    }

    fn gui(&mut self, _application: &Application, context: &egui::Context) {
        if self.controls.is_empty() {
            return;
        }
        egui::Window::new("Settings").show(context, |ui| {
            Python::attach(|py| {
                for slider in self.controls.iter_mut() {
                    let mut slider_mut = slider.borrow_mut(py);
                    let minimum = slider_mut.minimum;
                    let maximum = slider_mut.maximum;
                    ui.label(&slider_mut.name);
                    ui.add(Slider::new(&mut slider_mut.value, minimum..=maximum));
                }
            });
        });
    }
}

#[pyclass(unsendable)]
pub struct PyApplication {
    pub application: Option<Application>,
    pub event_loop_proxy: EventLoopProxy<CustomEvent>,
    pub renderables: Vec<Box<dyn Renderable>>,
    pub controls: Vec<Py<PySlider>>,
    pub update: Option<Py<PyFunction>>,
    auto_screenshot: Option<AutoScreenshot>,
}

#[pymethods]
impl PyApplication {
    #[new]
    fn new(event_loop: &mut PyEventLoop) -> Self {
        let mut application = Self {
            application: None,
            renderables: Vec::new(),
            controls: Vec::new(),
            update: None,
            event_loop_proxy: event_loop
                .event_loop
                .as_ref()
                .expect("Event loop already consumed")
                .create_proxy(),
            auto_screenshot: AutoScreenshot::from_env(),
        };
        if let Some(inner_loop) = event_loop.event_loop.as_mut() {
            inner_loop.pump_app_events(Some(Duration::from_secs(0)), &mut application);
        }
        application
    }
}
impl ApplicationHandler<CustomEvent> for PyApplication {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = match create_window(event_loop) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to create window: {e}");
                return;
            }
        };
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
        let Some(ref mut update) = self.update else {
            return;
        };
        application.window_event(window_id, &event);
        match event {
            WindowEvent::RedrawRequested => {
                if let Some(auto) = self.auto_screenshot.as_mut() {
                    if !auto.captured && auto.frames_rendered >= auto.capture_after_frames {
                        application.request_screenshot(auto.path.clone());
                        auto.captured = true;
                    }
                }
                application.update();
                let mut sim = PySimulation {
                    renderables: &self.renderables,
                    controls: &mut self.controls,
                };
                application.render(&mut sim);
                application.window.request_redraw();
                if let Some(auto) = self.auto_screenshot.as_mut() {
                    auto.frames_rendered = auto.frames_rendered.saturating_add(1);
                    if auto.captured {
                        event_loop.exit();
                        return;
                    }
                }
                Python::attach(|py| {
                    let result = update.call(py, (), None);
                    if let Err(err) = result {
                        println!("Could not call update: {err:?}");
                        println!("{}", err.traceback(py).unwrap().format().unwrap());
                    }
                });
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: CustomEvent) {
        match event {
            CustomEvent::Application(application) => self.application = Some(*application),
            CustomEvent::DropEvent(_) => {}
        }
    }
}
