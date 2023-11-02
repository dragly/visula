use bytemuck::{Pod, Zeroable};
use itertools::Itertools;

use pyo3::types::PyFunction;
use pyo3::{buffer::PyBuffer, prelude::*};

use visula::{
    create_event_loop, create_window, initialize_logger, Application, CustomEvent, Expression,
    InstanceBuffer, LineDelegate, Lines, PyLineDelegate, PySphereDelegate, RenderData, Renderable,
    RunConfig, SphereDelegate, Spheres,
};
use visula_core::glam::{Vec3, Vec4};
use visula_derive::Instance;
use wgpu::Color;

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
    position: f32,
    _padding: [f32; 3],
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

    fn cos(&self) -> PyExpression {
        Self {
            inner: self.inner.cos(),
        }
    }
    fn sin(&self) -> PyExpression {
        Self {
            inner: self.inner.sin(),
        }
    }
    fn tan(&self) -> PyExpression {
        Self {
            inner: self.inner.tan(),
        }
    }
}

#[pyclass(unsendable)]
struct PyInstanceBuffer {
    inner: InstanceBuffer<PointData>,
}

#[pymethods]
impl PyInstanceBuffer {
    #[new]
    fn new(py: Python, pyapplication: &PyApplication, obj: PyObject) -> Self {
        let PyApplication { application, .. } = pyapplication;
        let x = obj.extract::<PyBuffer<f64>>(py).expect("Could not extract");
        let buffer = InstanceBuffer::<PointData>::new(&application.device);
        let point_data: Vec<PointData> = x
            .to_vec(py)
            .expect("Cannot convert to vec")
            .iter()
            .copied()
            //.chunks(3)
            .into_iter()
            .map(|x| PointData {
                position: x as f32,
                _padding: Default::default(),
            })
            .collect();
        buffer.update(&application.device, &application.queue, &point_data);
        PyInstanceBuffer { inner: buffer }
    }

    fn update_buffer(
        &self,
        py: Python,
        pyapplication: &PyApplication,
        data: PyObject,
    ) -> PyResult<()> {
        let PyApplication { application, .. } = pyapplication;
        let x = data
            .extract::<PyBuffer<f64>>(py)
            .expect("Could not extract");
        let point_data: Vec<PointData> = x
            .to_vec(py)
            .expect("Cannot convert to vec")
            .iter()
            .copied()
            .into_iter()
            .map(|x| PointData {
                position: x as f32,
                _padding: Default::default(),
            })
            .collect();
        self.inner
            .update(&application.device, &application.queue, &point_data);
        Ok(())
    }

    fn instance(&self) -> PyExpression {
        PyExpression {
            inner: self.inner.instance().position,
        }
    }
}

#[pyfunction]
fn vec3(x: &PyExpression, y: &PyExpression, z: &PyExpression) -> PyExpression {
    PyExpression {
        inner: Expression::Vector3 {
            x: x.inner.clone().into(),
            y: y.inner.clone().into(),
            z: z.inner.clone().into(),
        },
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
        let buffer = InstanceBuffer::<PointData>::new(&application.device);
        let instance = buffer.instance();
        let point_data: Vec<PointData> = x
            .to_vec(py)
            .expect("Cannot convert to vec")
            .iter()
            .copied()
            .into_iter()
            .map(|x| PointData {
                position: x as f32,
                _padding: Default::default(),
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
        let buffer = InstanceBuffer::<PointData>::new(&application.device);
        let instance = buffer.instance();
        let point_data: Vec<PointData> = x
            .to_vec(py)
            .expect("Cannot convert to vec")
            .iter()
            .copied()
            .into_iter()
            .map(|x| PointData {
                position: x,
                _padding: Default::default(),
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

#[pyclass(unsendable)]
pub struct PyApplication {
    application: Application,
}

#[pyclass(unsendable)]
pub struct PyEventLoop {
    event_loop: EventLoop<CustomEvent>,
}

#[pymethods]
impl PyEventLoop {
    #[new]
    fn new() -> Self {
        initialize_logger();
        let event_loop = create_event_loop();
        Self { event_loop }
    }
}

#[pymethods]
impl PyApplication {
    #[new]
    fn new(event_loop: &PyEventLoop) -> Self {
        let window = create_window(
            RunConfig {
                canvas_name: "none".to_owned(),
            },
            &event_loop.event_loop,
        );
        let application = pollster::block_on(async { Application::new(window).await });
        Self { application }
    }
}

#[pyfunction]
fn show(
    py: Python,
    py_event_loop: &mut PyEventLoop,
    py_application: &PyCell<PyApplication>,
    renderables: Vec<PyObject>,
    update: &PyFunction,
) -> PyResult<()> {
    // TODO consider making the application retained so that not all the wgpu initialization needs
    // to be re-done

    let spheres_list: Vec<Box<dyn Renderable>> = {
        let application = py_application.borrow_mut();
        renderables
            .iter()
            .map(|renderable| -> Box<dyn Renderable> {
                // TODO automate the conversion
                if let Ok(pysphere) = renderable.extract::<PySphereDelegate>(py) {
                    return Box::new(
                        Spheres::new(
                            &application.application.rendering_descriptor(),
                            &SphereDelegate {
                                position: convert(py, &application, pysphere.position).inner,
                                radius: convert(py, &application, pysphere.radius).inner,
                                color: convert(py, &application, pysphere.color).inner,
                            },
                        )
                        .expect("Failed to create spheres"),
                    );
                }
                if let Ok(pylines) = renderable.extract::<PyLineDelegate>(py) {
                    return Box::new(
                        Lines::new(
                            &application.application.rendering_descriptor(),
                            &LineDelegate {
                                start: convert(py, &application, pylines.start).inner,
                                end: convert(py, &application, pylines.end).inner,
                                width: convert(py, &application, pylines.width).inner,
                                alpha: convert(py, &application, pylines.alpha).inner,
                            },
                        )
                        .expect("Failed to create spheres"),
                    );
                }
                unimplemented!("TODO")
            })
            .collect_vec()
    };

    let PyEventLoop { event_loop } = py_event_loop;

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::RedrawEventsCleared => {
                py_application.borrow().application.window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let application = &py_application.borrow().application;
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
                let result = update.call((), None);
                if let Err(err) = result {
                    println!("Could not call update: {:?}", err);
                    println!("{}", err.traceback(py).unwrap().format().unwrap());
                }

                py_application.borrow_mut().application.update();
            }
            event => {
                py_application
                    .borrow_mut()
                    .application
                    .handle_event(&event, control_flow);
            }
        }
    });
    Ok(())
}

#[pyfunction]
fn testme(update: &PyFunction) {
    println!("Called testme");
    let result = update.call((), None);
    if let Err(err) = result {
        println!("Could not call update: {:?}", err);
    }
}

#[pyfunction]
fn testyou() {
    println!("Called testyou");
}

#[pymodule]
#[pyo3(name = "_visula_pyo3")]
fn visula_pyo3(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(show, m)?)?;
    m.add_function(wrap_pyfunction!(testme, m)?)?;
    m.add_function(wrap_pyfunction!(testyou, m)?)?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_function(wrap_pyfunction!(vec3, m)?)?;
    m.add_class::<PyLineDelegate>()?;
    m.add_class::<PySphereDelegate>()?;
    m.add_class::<PyExpression>()?;
    m.add_class::<PyApplication>()?;
    m.add_class::<PyEventLoop>()?;
    m.add_class::<PyInstanceBuffer>()?;
    Ok(())
}
