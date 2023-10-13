use itertools::Itertools;
use std::sync::{Arc, Mutex};
use winit::platform::run_return::EventLoopExtRunReturn;

use bytemuck::{Pod, Zeroable};
use pyo3::{buffer::PyBuffer, prelude::*};
use visula::{
    application, create_event_loop, create_window, initialize_event_loop_and_window,
    initialize_logger, Application, CustomEvent, Expression, InstanceBuffer, LineDelegate, Lines,
    RenderData, RunConfig, SphereDelegate, Spheres,
};
use visula_core::glam::{Vec3, Vec4};
use visula_derive::Instance;
use wgpu::Color;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct SphereData {
    position: [f32; 3],
    _padding: f32,
}

#[derive(Debug)]
struct Error {}

async fn run(spheres: Spheres) {}

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct PointData {
    position: Vec3,
    _padding: f32,
}

#[pyclass(name = "Points", unsendable)]
struct PyPoints {
    buffer: InstanceBuffer<PointData>,
    pybuffer: PyBuffer<f64>,
    instance: PointDataInstance,
    #[pyo3(get)]
    position: PyExpression,
}

#[pymethods]
impl PyPoints {
    #[new]
    fn new(positions: PyBuffer<f64>) -> Self {
        let buffer = InstanceBuffer::<PointData>::new();
        let instance = buffer.instance();
        let position = PyExpression {
            inner: instance.position.clone(),
        };
        Self {
            pybuffer: positions,
            buffer,
            instance,
            position,
        }
    }

    fn update(&mut self, py: Python, buffer: PyBuffer<f64>) {
        //self.buffer.update(device, queue, data);
        self.pybuffer = buffer;
    }
}

#[pyclass(name = "Expression", unsendable)]
#[derive(Clone)]
struct PyExpression {
    inner: Expression,
}

#[pyclass(name = "Spheres", unsendable)]
#[derive(Clone)]
struct PySpheres {
    #[pyo3(get, set)]
    position: PyExpression,
    #[pyo3(get, set)]
    radius: PyExpression,
    #[pyo3(get, set)]
    color: PyExpression,
}

fn convert(py: Python, obj: PyObject) -> PyExpression {
    if let Ok(x) = obj.extract::<f32>(py) {
        return PyExpression { inner: x.into() };
    }
    if let Ok(x) = obj.extract::<Vec<f32>>(py) {
        if x.len() == 3 {
            return PyExpression {
                inner: Vec3::new(x[0], x[1], x[2]).into(),
            };
        } else if x.len() == 4 {
            return PyExpression {
                inner: Vec4::new(x[0], x[1], x[2], x[3]).into(),
            };
        } else {
            panic!("Vec of length {} are not supported", x.len());
        }
    }
    obj.extract(py).expect("Extract failed")
}

#[pymethods]
impl PySpheres {
    #[new]
    fn new(py: Python, position: PyObject, radius: PyObject, color: PyObject) -> Self {
        Self {
            position: convert(py, position),
            radius: convert(py, radius),
            color: convert(py, color),
        }
    }
}

#[pyclass(name = "Application", unsendable)]
struct PyApplication {
    event_loop: EventLoop<CustomEvent>,
}

#[pymethods]
impl PyApplication {
    #[new]
    fn new() -> Self {
        initialize_logger();
        let event_loop = create_event_loop();
        Self { event_loop }
    }
}

#[pyfunction]
fn spawn(
    py: Python,
    pyapplication: &mut PyApplication,
    pyspheres: PySpheres,
    points: &mut PyPoints,
) -> PyResult<()> {
    let PyApplication { event_loop } = pyapplication;
    let window = create_window(
        RunConfig {
            canvas_name: "none".to_owned(),
        },
        event_loop,
    );
    // TODO consider making the application retained so that not all the wgpu initialization needs
    // to be re-done
    let mut application = pollster::block_on(async { Application::new(window).await });
    let spheres = Spheres::new(
        &application.rendering_descriptor(),
        &SphereDelegate {
            position: pyspheres.position.inner,
            radius: pyspheres.radius.inner,
            color: pyspheres.color.inner,
        },
    )
    .expect("Failed to create spheres");

    let point_data: Vec<PointData> = points
        .pybuffer
        .to_vec(py)
        .expect("Cannot convert to vec")
        .iter()
        .map(|&v| v as f32)
        .chunks(3)
        .into_iter()
        .map(|x| {
            let v: Vec<f32> = x.collect();
            PointData {
                position: Vec3::new(v[0], v[1], v[2]),
                _padding: Default::default(),
            }
        })
        .collect();

    points
        .buffer
        .update(&application.device, &application.queue, &point_data);

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::RedrawEventsCleared => {
                application.window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let frame = application.next_frame();
                let mut encoder = application.encoder();

                {
                    let view = application.begin_render_pass(&frame, &mut encoder, Color::BLACK);
                    let mut render_data = RenderData {
                        view: &view,
                        depth_texture: &application.depth_texture,
                        encoder: &mut encoder,
                        camera: &application.camera,
                    };
                    spheres.render(&mut render_data);
                }

                application.queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::LoopDestroyed => {
                println!("Bye!");
            }
            Event::MainEventsCleared => {
                application.update();
            }
            event => {
                application.handle_event(&event, control_flow);
            }
        }
    });
    Ok(())
}

#[pymodule]
#[pyo3(name="_visula_pyo3")]
fn visula_pyo3(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(spawn, m)?)?;
    m.add_class::<PyExpression>()?;
    m.add_class::<PyPoints>()?;
    m.add_class::<PySpheres>()?;
    m.add_class::<PyApplication>()?;
    Ok(())
}
