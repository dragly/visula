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
    position: Vec3,
    _padding: f32,
}

// TODO generate the renderables from the delegates

fn convert(py: Python, application: &Application, obj: &PyObject) -> Expression {
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

        buffer.update(&application.device, &application.queue, &point_data);

        return instance.position;
    }
    if let Ok(x) = obj.extract::<f32>(py) {
        return x.into();
    }
    if let Ok(x) = obj.extract::<Vec<f32>>(py) {
        if x.len() == 3 {
            return Vec3::new(x[0], x[1], x[2]).into();
        } else if x.len() == 4 {
            return Vec4::new(x[0], x[1], x[2], x[3]).into();
        } else {
            panic!("Vec of length {} are not supported", x.len());
        }
    }
    unimplemented!("No support for obj")
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
fn show(
    py: Python,
    pyapplication: &mut PyApplication,
    renderables: Vec<PyObject>,
    callback: &PyFunction,
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

    let spheres_list: Vec<Box<dyn Renderable>> = renderables
        .iter()
        .map(|renderable| -> Box<dyn Renderable> {
            // TODO automate the conversion
            if let Ok(pysphere) = renderable.extract::<PySphereDelegate>(py) {
                return Box::new(
                    Spheres::new(
                        &application.rendering_descriptor(),
                        &SphereDelegate {
                            position: convert(py, &application, &pysphere.position),
                            radius: convert(py, &application, &pysphere.radius),
                            color: convert(py, &application, &pysphere.color),
                        },
                    )
                    .expect("Failed to create spheres"),
                );
            }
            if let Ok(pylines) = renderable.extract::<PyLineDelegate>(py) {
                return Box::new(
                    Lines::new(
                        &application.rendering_descriptor(),
                        &LineDelegate {
                            start: convert(py, &application, &pylines.start),
                            end: convert(py, &application, &pylines.end),
                            width: convert(py, &application, &pylines.width),
                            alpha: convert(py, &application, &pylines.alpha),
                        },
                    )
                    .expect("Failed to create spheres"),
                );
            }
            unimplemented!("TODO")
        })
        .collect_vec();

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
                    match event {
                        Event::WindowEvent {
                            event: window_event,
                            ..
                        } => match window_event {
                            WindowEvent::MouseInput {
                                state: ElementState::Released,
                                button: MouseButton::Left,
                                ..
                            } => {
                                let kwargs = PyDict::new(py);
                                kwargs.set_item("first", "hello").expect("Failed to insert");
                                kwargs.set_item("second", "world").expect("Failed to insert");
                                callback
                                    .call((), Some(kwargs))
                                    .expect("Could not call callback");
                            }
                            _ => {}
                        },
                        _ => {}
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
    m.add_class::<PyLineDelegate>()?;
    m.add_class::<PySphereDelegate>()?;
    m.add_class::<PyApplication>()?;
    Ok(())
}
