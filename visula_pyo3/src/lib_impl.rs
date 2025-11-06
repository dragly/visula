mod application;
mod event_loop;

use application::PyApplication;
use event_loop::PyEventLoop;

use bytemuck::{Pod, Zeroable};
use numpy::ndarray::Axis;
use numpy::PyReadonlyArray2;

use itertools::Itertools;
use std::cell::RefCell;

use std::rc::Rc;

use pyo3::types::PyFunction;
use pyo3::{buffer::PyBuffer, prelude::*};

use visula::{
    Expression, InstanceBuffer, LineDelegate, Lines, PyLineDelegate, PySphereDelegate, Renderable,
    SphereDelegate, Spheres,
};
use visula_core::glam::{Vec3, Vec4};
use visula_core::uuid::Uuid;
use visula_core::{UniformBufferInner, UniformField};
use visula_derive::Instance;
use wgpu::BufferUsages;

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct SphereData {
    position: [f32; 3],
    _padding: f32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct PointData {
    position: [f32; 3],
    _padding: f32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct FloatData {
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

#[pyclass]
#[derive(Clone, Debug)]
pub struct PySlider {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub value: f32,
    #[pyo3(get, set)]
    pub minimum: f32,
    #[pyo3(get, set)]
    pub maximum: f32,
    #[pyo3(get, set)]
    pub step: f32,
}

#[pymethods]
impl PySlider {
    #[new]
    fn new(name: &str, value: f32, minimum: f32, maximum: f32, step: f32) -> Self {
        Self {
            name: name.to_owned(),
            value,
            minimum,
            maximum,
            step,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct PyUniformField {
    name: String,
    ty: String,
    size: usize,
}

#[pymethods]
impl PyUniformField {
    #[new]
    fn new(name: &str, ty: &str, size: usize) -> Self {
        PyUniformField {
            name: name.to_owned(),
            ty: ty.to_owned(),
            size,
        }
    }
}

#[pyclass(unsendable)]
struct PyUniformBuffer {
    inner: Rc<RefCell<UniformBufferInner>>,
    fields: Vec<PyUniformField>,
    name: String,
    size: usize,
    queue: wgpu::Queue,
}

#[pymethods]
impl PyUniformBuffer {
    #[new]
    fn new(
        _py: Python,
        pyapplication: &PyApplication,
        fields: Vec<PyUniformField>,
        name: &str,
    ) -> Self {
        let PyApplication { application, .. } = pyapplication;
        let Some(application) = application else {
            panic!("Application not yet initialized");
        };

        let size = fields.iter().fold(0, |acc, field| acc + field.size);

        let usage = BufferUsages::UNIFORM | BufferUsages::COPY_DST;
        let buffer = application.device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            size: size as u64,
            label: Some(name),
            usage,
        });

        let bind_group_layout =
            application
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let bind_group = application
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

        Self {
            inner: Rc::new(RefCell::new(UniformBufferInner {
                label: name.into(),
                buffer,
                handle: Uuid::new_v4(),
                bind_group,
                bind_group_layout: Rc::new(bind_group_layout),
            })),
            fields,
            name: name.into(),
            size,
            queue: application.queue.clone(),
        }
    }

    fn update(&self, py: Python, buffer: PyBuffer<u8>) {
        let data = buffer.to_vec(py).expect("Could not turn PyBuffer into vec");
        let inner = self.inner.borrow_mut();
        self.queue.write_buffer(&inner.buffer, 0, &data);
    }

    fn field(&self, field_index: usize) -> PyExpression {
        let fields = self.fields.clone();
        let name = self.name.clone();
        let size = self.size as u32;
        let integrate = move |inner: &std::rc::Rc<
            std::cell::RefCell<visula_core::UniformBufferInner>,
        >,
                              handle: &::visula_core::uuid::Uuid,
                              module: &mut ::visula_core::naga::Module,
                              binding_builder: &mut visula_core::BindingBuilder,
                              bind_group_layout: &std::rc::Rc<
            ::visula_core::wgpu::BindGroupLayout,
        >| {
            if binding_builder.uniforms.contains_key(&handle.clone()) {
                return;
            };

            let entry_point_index = binding_builder.entry_point_index;
            let _previous_shader_location_offset = binding_builder.shader_location_offset;
            let _slot = binding_builder.current_slot;
            let bind_group = binding_builder.current_bind_group;

            let mut uniform_field_definitions = vec![];
            let mut offset = 0;
            for field in &fields {
                let naga_type_inner = match field.ty.as_ref() {
                    "float" => match field.size {
                        4 => naga::TypeInner::Scalar(naga::Scalar {
                            kind: naga::ScalarKind::Float,
                            width: 4,
                        }),
                        t => unimplemented!("Float with size {:?} is not yet implemented", t),
                    },
                    t => unimplemented!("Field type {:?} is not yet implemented", t),
                };

                let naga_type = naga::Type {
                    name: None,
                    inner: naga_type_inner,
                };

                let field_type = module
                    .types
                    .insert(naga_type, ::visula_core::naga::Span::default());
                uniform_field_definitions.push(::visula_core::naga::StructMember {
                    name: Some(field.name.clone()),
                    ty: field_type,
                    binding: None,
                    offset,
                });
                offset += field.size as u32;
            }

            let uniform_type = module.types.insert(
                ::visula_core::naga::Type {
                    name: Some(name.clone()), // TODO increment a counter to avoid collisions?
                    inner: ::visula_core::naga::TypeInner::Struct {
                        members: uniform_field_definitions,
                        span: size,
                    },
                },
                ::visula_core::naga::Span::default(),
            );
            let uniform_variable = module.global_variables.append(
                ::visula_core::naga::GlobalVariable {
                    name: Some(name.to_lowercase()),
                    binding: Some(::visula_core::naga::ResourceBinding {
                        group: bind_group,
                        binding: 0,
                    }),
                    space: ::visula_core::naga::AddressSpace::Uniform,
                    ty: uniform_type,
                    init: None,
                },
                ::visula_core::naga::Span::default(),
            );
            let uniform_expression = module.entry_points[entry_point_index]
                .function
                .expressions
                .append(
                    ::visula_core::naga::Expression::GlobalVariable(uniform_variable),
                    ::visula_core::naga::Span::default(),
                );

            binding_builder.uniforms.insert(
                *handle,
                visula_core::UniformBinding {
                    expression: uniform_expression,
                    bind_group_layout: bind_group_layout.clone(),
                    inner: inner.clone(),
                },
            );
            binding_builder.current_bind_group += 1;
        };

        PyExpression {
            inner: Expression::UniformField(UniformField {
                field_index,
                bind_group_layout: self.inner.borrow().bind_group_layout.clone(),
                buffer_handle: self.inner.borrow().handle,
                inner: self.inner.clone(),
                integrate_buffer: Rc::new(RefCell::new(integrate)),
            }),
        }
    }
}

impl PyUniformBuffer {
    // TODO: Create instance on Rust side that
    // includes the relevant code to generate the shader
}

#[pyclass(unsendable)]
struct PyInstanceBuffer {
    inner: InstanceBuffer<FloatData>,
}

#[pymethods]
impl PyInstanceBuffer {
    #[new]
    fn new(py: Python, pyapplication: &PyApplication, obj: Py<PyAny>) -> Self {
        let PyApplication { application, .. } = pyapplication;
        let Some(application) = application else {
            panic!("Application not yet initialized");
        };
        let x = obj.extract::<PyBuffer<f64>>(py).expect("Could not extract");
        let buffer = InstanceBuffer::<FloatData>::new(&application.device);
        let point_data: Vec<FloatData> = x
            .to_vec(py)
            .expect("Cannot convert to vec")
            .iter()
            .copied()
            .map(|x| FloatData {
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
        data: Py<PyAny>,
    ) -> PyResult<()> {
        let PyApplication { application, .. } = pyapplication;
        let Some(application) = application else {
            panic!("Application not yet initialized");
        };
        let x = data
            .extract::<PyBuffer<f64>>(py)
            .expect("Could not extract");
        let point_data: Vec<FloatData> = x
            .to_vec(py)
            .expect("Cannot convert to vec")
            .iter()
            .copied()
            .map(|x| FloatData {
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
fn convert(py: Python, pyapplication: &PyApplication, obj: Py<PyAny>) -> PyExpression {
    let PyApplication { application, .. } = pyapplication;
    let Some(application) = application else {
        panic!("Application not yet initialized");
    };
    if let Ok(expr) = obj.extract::<PyExpression>(py) {
        return expr;
    }
    if let Ok(inner) = obj.getattr(py, "inner") {
        if let Ok(expr) = inner.extract::<PyExpression>(py) {
            return expr;
        }
    }
    if let Ok(x) = obj.extract::<PyReadonlyArray2<f64>>(py) {
        let array = x.as_array();
        let major_axis = {
            if let Some(index) = array.shape().iter().position(|&size| size == 3) {
                match index {
                    0 => 1,
                    1 => 0,
                    i => panic!("Got index {i} in what was supposed to be a 2D array"),
                }
            } else {
                panic!("Must have a dimensions with three elements");
            }
        };
        let buffer = InstanceBuffer::<PointData>::new(&application.device);
        let instance = buffer.instance();
        let point_data: Vec<PointData> = x
            .as_array()
            .axis_iter(Axis(major_axis))
            .map(|v| PointData {
                position: [v[0] as f32, v[1] as f32, v[2] as f32],
                _padding: Default::default(),
            })
            .collect();

        buffer.update(&application.device, &application.queue, &point_data);

        return PyExpression {
            inner: instance.position,
        };
    }
    if let Ok(x) = obj.extract::<PyBuffer<f64>>(py) {
        // TODO optimize the case where the same PyBuffer has already
        // been written to a wgpu Buffer, for instance by creating
        // a cache
        let buffer = InstanceBuffer::<FloatData>::new(&application.device);
        let instance = buffer.instance();
        let point_data: Vec<FloatData> = x
            .to_vec(py)
            .expect("Cannot convert to vec")
            .iter()
            .copied()
            .map(|x| FloatData {
                position: x as f32,
                _padding: Default::default(),
            })
            .collect();

        buffer.update(&application.device, &application.queue, &point_data);

        return PyExpression {
            inner: instance.position,
        };
    }
    if let Ok(x) = obj.extract::<PyBuffer<f32>>(py) {
        // TODO optimize the case where the same PyBuffer has already
        // been written to a wgpu Buffer, for instance by creating
        // a cache
        let buffer = InstanceBuffer::<FloatData>::new(&application.device);
        let instance = buffer.instance();
        let point_data: Vec<FloatData> = x
            .to_vec(py)
            .expect("Cannot convert to vec")
            .iter()
            .copied()
            .map(|x| FloatData {
                position: x,
                _padding: Default::default(),
            })
            .collect();

        buffer.update(&application.device, &application.queue, &point_data);

        return PyExpression {
            inner: instance.position,
        };
    }
    if let Ok(x) = obj.extract::<f32>(py) {
        return PyExpression { inner: x.into() };
    }
    if let Ok(x) = obj.extract::<Vec<f32>>(py) {
        if x.len() == 3 {
            return PyExpression {
                inner: Vec3::new(x[0], x[1], x[2]).into(),
            };
        }
        if x.len() == 4 {
            return PyExpression {
                inner: Vec4::new(x[0], x[1], x[2], x[3]).into(),
            };
        }
        unimplemented!("Vec of length {} are not supported", x.len());
    }
    unimplemented!("No support for obj: {obj}")
}

#[pyfunction]
fn show(
    py: Python,
    py_application: &Bound<PyApplication>,
    py_renderables: Vec<Py<PyAny>>,
    update: Py<PyFunction>,
    controls: Vec<Py<PySlider>>,
) -> PyResult<()> {
    {
        let mut py_application_mut = py_application.borrow_mut();
        let Some(ref application) = py_application_mut.application else {
            panic!("Application not yet initialized");
        };
        application.window.request_redraw();
        let renderables: Vec<Box<dyn Renderable>> = py_renderables
            .iter()
            .map(|renderable| -> Box<dyn Renderable> {
                // TODO automate the conversion
                if let Ok(pysphere) = renderable.extract::<PySphereDelegate>(py) {
                    return Box::new(
                        Spheres::new(
                            &application.rendering_descriptor(),
                            &SphereDelegate {
                                position: convert(py, &py_application_mut, pysphere.position).inner,
                                radius: convert(py, &py_application_mut, pysphere.radius).inner,
                                color: convert(py, &py_application_mut, pysphere.color).inner,
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
                                start: convert(py, &py_application_mut, pylines.start).inner,
                                end: convert(py, &py_application_mut, pylines.end).inner,
                                width: convert(py, &py_application_mut, pylines.width).inner,
                                color: convert(py, &py_application_mut, pylines.color).inner,
                            },
                        )
                        .expect("Failed to create spheres"),
                    );
                }
                unimplemented!("TODO")
            })
            .collect_vec();

        py_application_mut.renderables = renderables;
        py_application_mut.update = Some(update);
        py_application_mut.controls = controls;
    }

    Ok(())
}

#[pymodule]
#[pyo3(name = "_visula_pyo3")]
fn visula_pyo3(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(show, m)?)?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_function(wrap_pyfunction!(vec3, m)?)?;
    m.add_class::<PyLineDelegate>()?;
    m.add_class::<PySphereDelegate>()?;
    m.add_class::<PyExpression>()?;
    m.add_class::<PyApplication>()?;
    m.add_class::<PyEventLoop>()?;
    m.add_class::<PyInstanceBuffer>()?;
    m.add_class::<PyUniformBuffer>()?;
    m.add_class::<PyUniformField>()?;
    m.add_class::<PySlider>()?;
    Ok(())
}
