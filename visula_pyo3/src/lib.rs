use bytemuck::{Pod, Zeroable};
use itertools::Itertools;

use pyo3::types::{PyDict, PyFunction};
use pyo3::{buffer::PyBuffer, prelude::*};

use visula::{
    create_event_loop, create_window, initialize_logger, Application, CustomEvent, Expression,
    InstanceBuffer, LineDelegate, Lines, PyLineDelegate, PySphereDelegate, RenderData, Renderable,
    RunConfig, SphereDelegate, Spheres,
};
use visula_core::glam::{Vec3, Vec4};
use visula_derive::Instance;
use wgpu::Color;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct SphereData {
    position: [f32; 3],
    _padding: f32,
}

#[derive(Debug)]
struct Error {}

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct PointData {
    position: [f32; 3],
    _padding: f32,
}

#[pyclass(name = "Expression", unsendable)]
#[derive(Clone)]
struct PyExpression {
    inner: Expression,
}

#[pymethods]
impl PyExpression {
    fn add(&self, other: &PyExpression) -> PyExpression {
        PyExpression {
            inner: self.inner.clone() + other.inner.clone(),
        }
    }

    fn sub(&self, other: &PyExpression) -> PyExpression {
        PyExpression {
            inner: self.inner.clone() - other.inner.clone(),
        }
    }

    fn mul(&self, other: &PyExpression) -> PyExpression {
        PyExpression {
            inner: self.inner.clone() * other.inner.clone(),
        }
    }

    fn truediv(&self, other: &PyExpression) -> PyExpression {
        PyExpression {
            inner: self.inner.clone() / other.inner.clone(),
        }
    }

    fn floordiv(&self, other: &PyExpression) -> PyExpression {
        PyExpression {
            inner: (self.inner.clone() / other.inner.clone()).floor(),
        }
    }

    fn modulo(&self, other: &PyExpression) -> PyExpression {
        PyExpression {
            inner: self.inner.clone() % other.inner.clone(),
        }
    }

    fn pow(&self, other: &PyExpression) -> PyExpression {
        PyExpression {
            inner: self.inner.clone().pow(other.inner.clone()),
        }
    }
}

#[pyfunction]
fn convert(py: Python, pyapplication: &PyApplication, obj: PyObject) -> PyExpression {
    let PyApplication { application, .. } = pyapplication;
    if let Ok(expr) = obj.extract::<PyExpression>(py) {
        return expr;
    }
    if let Ok(inner) = obj.getattr(py, "inner") {
        if let Ok(expr) = inner.extract::<PyExpression>(py) {
            return expr;
        }
    }
    if let Ok(x) = obj.extract::<PyBuffer<f64>>(py) {
        // TODO optimize the case where the same PyBuffer has already
        // been written to a wgpu Buffer, for instance by creating
        // a cache
        let mut buffer = InstanceBuffer::<PointData>::new(&application.device);
        let instance = buffer.instance();
        let point_data: Vec<PointData> = x
            .to_vec(py)
            .expect("Cannot convert to vec")
            .iter()
            .copied()
            .chunks(3)
            .into_iter()
            .map(|x| {
                let v: Vec<f64> = x.collect();
                PointData {
                    position: [v[0] as f32, v[1] as f32, v[2] as f32],
                    _padding: Default::default(),
                }
            })
            .collect();

        buffer.update(&application.device, &application.queue, &point_data);

        return PyExpression {
            inner: instance.position,
        };
    } else if let Ok(x) = obj.extract::<PyBuffer<f32>>(py) {
        // TODO optimize the case where the same PyBuffer has already
        // been written to a wgpu Buffer, for instance by creating
        // a cache
        let mut buffer = InstanceBuffer::<PointData>::new(&application.device);
        let instance = buffer.instance();
        let point_data: Vec<PointData> = x
            .to_vec(py)
            .expect("Cannot convert to vec")
            .iter()
            .copied()
            .chunks(3)
            .into_iter()
            .map(|x| {
                let v: Vec<f32> = x.collect();
                PointData {
                    position: [v[0], v[1], v[2]],
                    _padding: Default::default(),
                }
            })
            .collect();

        buffer.update(&application.device, &application.queue, &point_data);

        return PyExpression {
            inner: instance.position,
        };
    } else if let Ok(x) = obj.extract::<f32>(py) {
        return PyExpression { inner: x.into() };
    } else if let Ok(x) = obj.extract::<Vec<f32>>(py) {
        if x.len() == 3 {
            return PyExpression {
                inner: Vec3::new(x[0], x[1], x[2]).into(),
            };
        } else if x.len() == 4 {
            return PyExpression {
                inner: Vec4::new(x[0], x[1], x[2], x[3]).into(),
            };
        }
        unimplemented!("Vec of length {} are not supported", x.len());
    }
    unimplemented!("No support for obj: {obj}")
}

#[pyclass(name = "Application", unsendable)]
pub struct PyApplication {
    event_loop: EventLoop<CustomEvent>,
    application: Application,
}

#[pymethods]
impl PyApplication {
    #[new]
    fn new() -> Self {
        initialize_logger();
        let event_loop = create_event_loop();
        let window = create_window(
            RunConfig {
                canvas_name: "none".to_owned(),
            },
            &event_loop,
        );
        let application = pollster::block_on(async { Application::new(window).await });
        Self {
            event_loop,
            application,
        }
    }
}

#[pyfunction]
fn show(
    py: Python,
    pyapplication: &mut PyApplication,
    renderables: Vec<PyObject>,
    callback: &PyFunction,
) -> PyResult<()> {
    // TODO consider making the application retained so that not all the wgpu initialization needs
    // to be re-done

    let spheres_list: Vec<Box<dyn Renderable>> = renderables
        .iter()
        .map(|renderable| -> Box<dyn Renderable> {
            // TODO automate the conversion
            if let Ok(pysphere) = renderable.extract::<PySphereDelegate>(py) {
                return Box::new(
                    Spheres::new(
                        &pyapplication.application.rendering_descriptor(),
                        &SphereDelegate {
                            position: convert(py, &pyapplication, pysphere.position).inner,
                            radius: convert(py, &pyapplication, pysphere.radius).inner,
                            color: convert(py, &pyapplication, pysphere.color).inner,
                        },
                    )
                    .expect("Failed to create spheres"),
                );
            }
            if let Ok(pylines) = renderable.extract::<PyLineDelegate>(py) {
                return Box::new(
                    Lines::new(
                        &pyapplication.application.rendering_descriptor(),
                        &LineDelegate {
                            start: convert(py, &pyapplication, pylines.start).inner,
                            end: convert(py, &pyapplication, pylines.end).inner,
                            width: convert(py, &pyapplication, pylines.width).inner,
                            alpha: convert(py, &pyapplication, pylines.alpha).inner,
                        },
                    )
                    .expect("Failed to create spheres"),
                );
            }
            unimplemented!("TODO")
        })
        .collect_vec();

    let PyApplication {
        event_loop,
        application,
    } = pyapplication;

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
                    for spheres in &spheres_list {
                        spheres.render(&mut render_data);
                    }
                }

                application.queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::MainEventsCleared => {
                application.update();
            }
            event => {
                if !(application.handle_event(&event, control_flow)) {
                    if let Event::WindowEvent {
                        event:
                            WindowEvent::MouseInput {
                                state: ElementState::Released,
                                button: MouseButton::Left,
                                ..
                            },
                        ..
                    } = event
                    {
                        let kwargs = PyDict::new(py);
                        kwargs.set_item("first", "hello").expect("Failed to insert");
                        kwargs
                            .set_item("second", "world")
                            .expect("Failed to insert");
                        callback
                            .call((), Some(kwargs))
                            .expect("Could not call callback");
                    }
                }
            }
        }
    });
    Ok(())
}

#[pymodule]
#[pyo3(name = "_visula_pyo3")]
fn visula_pyo3(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(show, m)?)?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_class::<PyLineDelegate>()?;
    m.add_class::<PySphereDelegate>()?;
    m.add_class::<PyExpression>()?;
    m.add_class::<PyApplication>()?;
    Ok(())
}
