use log::{error, info};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use visula::{create_event_loop, initialize_logger, CustomEvent};

use winit::event_loop::EventLoop;

use super::application::PyApplication;

#[pyclass(unsendable)]
pub struct PyEventLoop {
    pub event_loop: Option<EventLoop<CustomEvent>>,
}

#[pymethods]
impl PyEventLoop {
    #[new]
    fn new() -> Self {
        initialize_logger();
        let event_loop = create_event_loop();
        info!("Created winit EventLoop");
        Self {
            event_loop: Some(event_loop),
        }
    }

    pub fn run(&mut self, py_application: &Bound<PyApplication>) -> PyResult<()> {
        info!("Starting winit event loop");
        let Some(event_loop) = self.event_loop.take() else {
            error!("Attempted to run event loop twice");
            return Err(PyRuntimeError::new_err(
                "Event loop already consumed or run() called twice",
            ));
        };

        let mut app = py_application.borrow_mut();
        if let Err(err) = event_loop.run_app(&mut *app) {
            error!("Event loop returned error: {err:?}");
            return Err(PyRuntimeError::new_err(format!(
                "Failed to run event loop: {err:?}"
            )));
        }
        info!("Event loop finished normally");
        Ok(())
    }
}
