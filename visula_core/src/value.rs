use std::{
    fmt::{Error, Formatter},
    ops::{Add, Deref, Div, Mul, Neg, Rem, Sub},
    rc::Rc,
};

use itertools::Itertools;
use naga::common::wgsl::{TryToWgsl, TypeContext};
use naga::proc::OverloadSet;
use naga::{GlobalVariable, ResourceBinding, Span};

use crate::{BindingBuilder, InstanceField, ShaderError, TextureField, UniformField};

#[derive(Clone)]
pub struct ExpressionInner {
    inner: Box<Expression>,
}

#[derive(Clone)]
pub enum Expression {
    BinaryOperator {
        left: ExpressionInner,
        right: ExpressionInner,
        operator: naga::BinaryOperator,
    },
    UnaryOperator {
        value: ExpressionInner,
        operator: naga::UnaryOperator,
    },
    Literal(naga::Literal),
    InstanceField(InstanceField),
    UniformField(UniformField),
    TextureField(TextureField),
    Vector2 {
        x: ExpressionInner,
        y: ExpressionInner,
    },
    Vector3 {
        x: ExpressionInner,
        y: ExpressionInner,
        z: ExpressionInner,
    },
    Vector4 {
        x: ExpressionInner,
        y: ExpressionInner,
        z: ExpressionInner,
        w: ExpressionInner,
    },
    Matrix4 {
        x_axis: ExpressionInner,
        y_axis: ExpressionInner,
        z_axis: ExpressionInner,
        w_axis: ExpressionInner,
    },
    Math {
        function: naga::MathFunction,
        arguments: Vec<ExpressionInner>,
    },
    UV,
    Normal,
    Position,
    InputColor,
    DirectionalLit(ExpressionInner),
    Lit(ExpressionInner),
    ToonLit(ExpressionInner),
    ViewDirection,
    Shared(Rc<SharedInner>),
}

pub struct SharedInner {
    id: uuid::Uuid,
    value: Expression,
    name: Option<String>,
}

fn math(function: naga::MathFunction, arguments: Vec<ExpressionInner>) -> Expression {
    Expression::Math {
        function,
        arguments,
    }
}

impl Expression {
    pub fn pow(&self, exponent: impl Into<ExpressionInner>) -> Expression {
        math(naga::MathFunction::Pow, vec![self.into(), exponent.into()])
    }

    pub fn exp(&self) -> Expression {
        math(naga::MathFunction::Exp, vec![self.into()])
    }

    pub fn length(&self) -> Expression {
        math(naga::MathFunction::Length, vec![self.into()])
    }

    pub fn floor(&self) -> Expression {
        math(naga::MathFunction::Floor, vec![self.into()])
    }
    pub fn cos(&self) -> Expression {
        math(naga::MathFunction::Cos, vec![self.into()])
    }
    pub fn sin(&self) -> Expression {
        math(naga::MathFunction::Sin, vec![self.into()])
    }
    pub fn tan(&self) -> Expression {
        math(naga::MathFunction::Tan, vec![self.into()])
    }
    pub fn sqrt(&self) -> Expression {
        math(naga::MathFunction::Sqrt, vec![self.into()])
    }
    pub fn abs(&self) -> Expression {
        math(naga::MathFunction::Abs, vec![self.into()])
    }
    pub fn fract(&self) -> Expression {
        math(naga::MathFunction::Fract, vec![self.into()])
    }
    pub fn ceil(&self) -> Expression {
        math(naga::MathFunction::Ceil, vec![self.into()])
    }
    pub fn round(&self) -> Expression {
        math(naga::MathFunction::Round, vec![self.into()])
    }
    pub fn sign(&self) -> Expression {
        math(naga::MathFunction::Sign, vec![self.into()])
    }
    pub fn log(&self) -> Expression {
        math(naga::MathFunction::Log, vec![self.into()])
    }
    pub fn normalize(&self) -> Expression {
        math(naga::MathFunction::Normalize, vec![self.into()])
    }
    pub fn min(&self, other: impl Into<ExpressionInner>) -> Expression {
        math(naga::MathFunction::Min, vec![self.into(), other.into()])
    }
    pub fn max(&self, other: impl Into<ExpressionInner>) -> Expression {
        math(naga::MathFunction::Max, vec![self.into(), other.into()])
    }
    pub fn dot(&self, other: impl Into<ExpressionInner>) -> Expression {
        math(naga::MathFunction::Dot, vec![self.into(), other.into()])
    }
    pub fn cross(&self, other: impl Into<ExpressionInner>) -> Expression {
        math(naga::MathFunction::Cross, vec![self.into(), other.into()])
    }
    pub fn distance(&self, other: impl Into<ExpressionInner>) -> Expression {
        math(
            naga::MathFunction::Distance,
            vec![self.into(), other.into()],
        )
    }
    pub fn atan2(&self, other: impl Into<ExpressionInner>) -> Expression {
        math(naga::MathFunction::Atan2, vec![self.into(), other.into()])
    }
    pub fn clamp(
        &self,
        low: impl Into<ExpressionInner>,
        high: impl Into<ExpressionInner>,
    ) -> Expression {
        math(
            naga::MathFunction::Clamp,
            vec![self.into(), low.into(), high.into()],
        )
    }
    pub fn mix(
        &self,
        other: impl Into<ExpressionInner>,
        amount: impl Into<ExpressionInner>,
    ) -> Expression {
        math(
            naga::MathFunction::Mix,
            vec![self.into(), other.into(), amount.into()],
        )
    }
    pub fn smoothstep(
        &self,
        edge_low: impl Into<ExpressionInner>,
        edge_high: impl Into<ExpressionInner>,
    ) -> Expression {
        math(
            naga::MathFunction::SmoothStep,
            vec![edge_low.into(), edge_high.into(), self.into()],
        )
    }
    pub fn step(&self, edge: impl Into<ExpressionInner>) -> Expression {
        math(naga::MathFunction::Step, vec![edge.into(), self.into()])
    }

    pub fn directional_lit(&self) -> Expression {
        Expression::DirectionalLit(self.into())
    }

    pub fn lit(&self) -> Expression {
        Expression::Lit(self.into())
    }

    pub fn toon_lit(&self) -> Expression {
        Expression::ToonLit(self.into())
    }

    /// Evaluates this expression once per shader stage, however often it is used.
    ///
    /// The same node used in both a vertex and a fragment expression is evaluated
    /// once in each stage.
    pub fn shared(&self) -> Expression {
        Expression::Shared(Rc::new(SharedInner {
            id: uuid::Uuid::new_v4(),
            value: self.clone(),
            name: None,
        }))
    }

    pub fn shared_with_name(&self, name: &str) -> Expression {
        Expression::Shared(Rc::new(SharedInner {
            id: uuid::Uuid::new_v4(),
            value: self.clone(),
            name: Some(name.to_string()),
        }))
    }
}

impl ExpressionInner {
    fn new(expression: Expression) -> ExpressionInner {
        ExpressionInner {
            inner: Box::new(expression),
        }
    }
}

impl<T> From<T> for ExpressionInner
where
    T: Into<Expression>,
{
    fn from(value: T) -> Self {
        ExpressionInner::new(value.into())
    }
}

impl Deref for ExpressionInner {
    type Target = Box<Expression>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub trait AsValue {
    fn as_value(&self) -> Expression;
}

impl AsValue for InstanceField {
    fn as_value(&self) -> Expression {
        Expression::InstanceField(self.clone())
    }
}

impl AsValue for UniformField {
    fn as_value(&self) -> Expression {
        Expression::UniformField(self.clone())
    }
}

fn load_local_variable(
    name: &str,
    module: &mut naga::Module,
    binding_builder: &BindingBuilder,
) -> Result<naga::Handle<naga::Expression>, ShaderError> {
    let entry_point = &mut module.entry_points[binding_builder.entry_point_index];
    let variable = entry_point
        .function
        .local_variables
        .fetch_if(|variable| variable.name == Some(name.into()))
        .ok_or_else(|| ShaderError::VariableNotFound(name.to_string()))?;
    let variable_expression = entry_point
        .function
        .expressions
        .fetch_if(|expression| match expression {
            naga::Expression::LocalVariable(v) => v == &variable,
            _ => false,
        })
        .ok_or_else(|| ShaderError::VariableNotFound(name.to_string()))?;
    Ok(entry_point.function.expressions.append(
        naga::Expression::Load {
            pointer: variable_expression,
        },
        Span::default(),
    ))
}

pub(crate) fn resolve_type(
    handle: naga::Handle<naga::Expression>,
    module: &naga::Module,
    binding_builder: &mut BindingBuilder,
) -> Result<naga::TypeInner, ShaderError> {
    let function = &module.entry_points[binding_builder.entry_point_index].function;
    let context = naga::proc::ResolveContext::with_locals(
        module,
        &function.local_variables,
        &function.arguments,
    );
    binding_builder
        .typifier
        .grow(handle, &function.expressions, &context)?;
    Ok(binding_builder.typifier.get(handle, &module.types).clone())
}

fn resolve_types(
    handles: &[naga::Handle<naga::Expression>],
    module: &naga::Module,
    binding_builder: &mut BindingBuilder,
) -> Result<Vec<naga::TypeInner>, ShaderError> {
    handles
        .iter()
        .map(|&handle| resolve_type(handle, module, binding_builder))
        .collect()
}

pub(crate) fn type_name(inner: &naga::TypeInner, module: &naga::Module) -> String {
    module
        .types
        .type_resolution_to_string(&naga::proc::TypeResolution::Value(inner.clone()))
}

fn type_names(types: &[naga::TypeInner], module: &naga::Module) -> String {
    types
        .iter()
        .map(|inner| type_name(inner, module))
        .join(", ")
}

const F32: naga::TypeInner = naga::TypeInner::Scalar(naga::Scalar::F32);

const fn vector(size: naga::VectorSize) -> naga::TypeInner {
    naga::TypeInner::Vector {
        size,
        scalar: naga::Scalar::F32,
    }
}

fn operands_match(
    operator: naga::BinaryOperator,
    left: &naga::TypeInner,
    right: &naga::TypeInner,
) -> bool {
    use naga::TypeInner::{Matrix, Scalar, Vector};
    let numeric = matches!(
        left.scalar().map(|scalar| scalar.kind),
        Some(naga::ScalarKind::Float | naga::ScalarKind::Sint | naga::ScalarKind::Uint)
    );
    match operator {
        naga::BinaryOperator::Add | naga::BinaryOperator::Subtract => numeric && left == right,
        naga::BinaryOperator::Divide | naga::BinaryOperator::Modulo => {
            numeric && !matches!(left, Matrix { .. }) && left == right
        }
        naga::BinaryOperator::Multiply => {
            numeric
                && left.scalar() == right.scalar()
                && match (left, right) {
                    (Scalar(_), _) | (_, Scalar(_)) => true,
                    (Vector { size: a, .. }, Vector { size: b, .. })
                    | (Matrix { columns: a, .. }, Vector { size: b, .. })
                    | (Vector { size: a, .. }, Matrix { rows: b, .. })
                    | (Matrix { columns: a, .. }, Matrix { rows: b, .. }) => a == b,
                    _ => false,
                }
        }
        _ => true,
    }
}

fn check_operands(
    operator: naga::BinaryOperator,
    left: naga::Handle<naga::Expression>,
    right: naga::Handle<naga::Expression>,
    module: &naga::Module,
    binding_builder: &mut BindingBuilder,
) -> Result<(), ShaderError> {
    let left = resolve_type(left, module, binding_builder)?;
    let right = resolve_type(right, module, binding_builder)?;
    if operands_match(operator, &left, &right) {
        return Ok(());
    }
    Err(ShaderError::UnsupportedOperandTypes {
        operator: naga::back::binary_operation_str(operator),
        left: type_name(&left, module),
        right: type_name(&right, module),
    })
}

fn check_operand(
    operator: naga::UnaryOperator,
    operand: naga::Handle<naga::Expression>,
    module: &naga::Module,
    binding_builder: &mut BindingBuilder,
) -> Result<(), ShaderError> {
    let operand = resolve_type(operand, module, binding_builder)?;
    let kind = match operand {
        naga::TypeInner::Scalar(scalar) | naga::TypeInner::Vector { scalar, .. } => {
            Some(scalar.kind)
        }
        _ => None,
    };
    let (symbol, supported) = match operator {
        naga::UnaryOperator::Negate => (
            "-",
            matches!(kind, Some(naga::ScalarKind::Float | naga::ScalarKind::Sint)),
        ),
        naga::UnaryOperator::LogicalNot => ("!", kind == Some(naga::ScalarKind::Bool)),
        naga::UnaryOperator::BitwiseNot => (
            "~",
            matches!(kind, Some(naga::ScalarKind::Sint | naga::ScalarKind::Uint)),
        ),
    };
    if supported {
        return Ok(());
    }
    Err(ShaderError::UnsupportedOperandType {
        operator: symbol,
        operand: type_name(&operand, module),
    })
}

fn is_color_vec4(
    function: &str,
    color: naga::Handle<naga::Expression>,
    module: &naga::Module,
    binding_builder: &mut BindingBuilder,
) -> Result<bool, ShaderError> {
    let color = resolve_type(color, module, binding_builder)?;
    if color == vector(naga::VectorSize::Quad) {
        Ok(true)
    } else if color == vector(naga::VectorSize::Tri) {
        Ok(false)
    } else {
        Err(ShaderError::UnsupportedArgumentTypes {
            function: function.to_string(),
            arguments: type_name(&color, module),
        })
    }
}

fn arguments_match(
    function: naga::MathFunction,
    arguments: &[naga::TypeInner],
    module: &naga::Module,
) -> bool {
    let overloads = arguments
        .iter()
        .enumerate()
        .fold(function.overloads(), |overloads, (index, argument)| {
            overloads.arg(index, argument, &module.types)
        });
    !overloads.is_empty() && arguments.len() >= overloads.min_arguments()
}

fn argument_splat(
    function: naga::MathFunction,
    arguments: &[naga::TypeInner],
    module: &naga::Module,
) -> Option<(naga::VectorSize, naga::Scalar)> {
    if !matches!(
        function,
        naga::MathFunction::Min
            | naga::MathFunction::Max
            | naga::MathFunction::Clamp
            | naga::MathFunction::Mix
            | naga::MathFunction::Step
            | naga::MathFunction::SmoothStep
            | naga::MathFunction::Pow
            | naga::MathFunction::Atan2
    ) {
        return None;
    }
    let (size, scalar) = arguments.iter().find_map(|argument| match *argument {
        naga::TypeInner::Vector { size, scalar } => Some((size, scalar)),
        _ => None,
    })?;
    let splatted: Vec<_> = arguments
        .iter()
        .map(|argument| match *argument {
            naga::TypeInner::Scalar(other) if other == scalar => {
                naga::TypeInner::Vector { size, scalar }
            }
            _ => argument.clone(),
        })
        .collect();
    arguments_match(function, &splatted, module).then_some((size, scalar))
}

fn compose(
    ty: naga::TypeInner,
    component: naga::TypeInner,
    components: &[ExpressionInner],
    module: &mut naga::Module,
    binding_builder: &mut BindingBuilder,
) -> Result<naga::Handle<naga::Expression>, ShaderError> {
    let handles = components
        .iter()
        .map(|component| component.setup(module, binding_builder))
        .collect::<Result<Vec<_>, _>>()?;
    let types = resolve_types(&handles, module, binding_builder)?;
    if types.iter().any(|inner| *inner != component) {
        return Err(ShaderError::UnsupportedArgumentTypes {
            function: type_name(&ty, module),
            arguments: type_names(&types, module),
        });
    }
    let ty = module.types.insert(
        naga::Type {
            name: None,
            inner: ty,
        },
        naga::Span::default(),
    );
    Ok(module.entry_points[binding_builder.entry_point_index]
        .function
        .expressions
        .append(
            naga::Expression::Compose {
                ty,
                components: handles,
            },
            naga::Span::default(),
        ))
}

fn splat_scalar_operand(
    operator: naga::BinaryOperator,
    left: &mut naga::Handle<naga::Expression>,
    right: &mut naga::Handle<naga::Expression>,
    module: &mut naga::Module,
    binding_builder: &mut BindingBuilder,
) -> Result<(), ShaderError> {
    if !matches!(
        operator,
        naga::BinaryOperator::Add
            | naga::BinaryOperator::Subtract
            | naga::BinaryOperator::Divide
            | naga::BinaryOperator::Modulo
    ) {
        return Ok(());
    }
    let (scalar, size) = match (
        resolve_type(*left, module, binding_builder)?,
        resolve_type(*right, module, binding_builder)?,
    ) {
        (naga::TypeInner::Vector { size, scalar }, naga::TypeInner::Scalar(other))
            if scalar == other =>
        {
            (right, size)
        }
        (naga::TypeInner::Scalar(other), naga::TypeInner::Vector { size, scalar })
            if scalar == other =>
        {
            (left, size)
        }
        _ => return Ok(()),
    };
    *scalar = module.entry_points[binding_builder.entry_point_index]
        .function
        .expressions
        .append(
            naga::Expression::Splat {
                size,
                value: *scalar,
            },
            naga::Span::default(),
        );
    Ok(())
}

fn find_function(
    module: &naga::Module,
    name: &str,
) -> Result<naga::Handle<naga::Function>, ShaderError> {
    module
        .functions
        .iter()
        .find(|(_, f)| f.name.as_deref() == Some(name))
        .map(|(handle, _)| handle)
        .ok_or_else(|| ShaderError::FunctionNotFound(name.to_string()))
}

impl Expression {
    pub fn setup(
        &self,
        module: &mut naga::Module,
        binding_builder: &mut BindingBuilder,
    ) -> Result<naga::Handle<naga::Expression>, ShaderError> {
        let val = self.clone();

        Ok(match val {
            Expression::Literal(inner) => module.entry_points[binding_builder.entry_point_index]
                .function
                .expressions
                .append(naga::Expression::Literal(inner), ::naga::Span::default()),
            Expression::Vector2 { x, y } => compose(
                vector(naga::VectorSize::Bi),
                F32,
                &[x, y],
                module,
                binding_builder,
            )?,
            Expression::Vector3 { x, y, z } => compose(
                vector(naga::VectorSize::Tri),
                F32,
                &[x, y, z],
                module,
                binding_builder,
            )?,
            Expression::Vector4 { x, y, z, w } => compose(
                vector(naga::VectorSize::Quad),
                F32,
                &[x, y, z, w],
                module,
                binding_builder,
            )?,
            Expression::Matrix4 {
                x_axis,
                y_axis,
                z_axis,
                w_axis,
            } => compose(
                naga::TypeInner::Matrix {
                    columns: naga::VectorSize::Quad,
                    rows: naga::VectorSize::Quad,
                    scalar: naga::Scalar::F32,
                },
                vector(naga::VectorSize::Quad),
                &[x_axis, y_axis, z_axis, w_axis],
                module,
                binding_builder,
            )?,
            Expression::BinaryOperator {
                left,
                right,
                operator,
            } => {
                let mut left_setup = left.setup(module, binding_builder)?;
                let mut right_setup = right.setup(module, binding_builder)?;
                splat_scalar_operand(
                    operator,
                    &mut left_setup,
                    &mut right_setup,
                    module,
                    binding_builder,
                )?;
                check_operands(operator, left_setup, right_setup, module, binding_builder)?;
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::Binary {
                            op: operator,
                            left: left_setup,
                            right: right_setup,
                        },
                        naga::Span::default(),
                    )
            }
            Expression::UnaryOperator { value, operator } => {
                let value_setup = value.setup(module, binding_builder)?;
                check_operand(operator, value_setup, module, binding_builder)?;
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::Unary {
                            expr: value_setup,
                            op: operator,
                        },
                        naga::Span::default(),
                    )
            }
            Expression::Math {
                function,
                arguments,
            } => {
                let mut handles = arguments
                    .iter()
                    .map(|argument| argument.setup(module, binding_builder))
                    .collect::<Result<Vec<_>, _>>()?;
                let types = resolve_types(&handles, module, binding_builder)?;
                if !arguments_match(function, &types, module) {
                    let Some((size, scalar)) = argument_splat(function, &types, module) else {
                        return Err(ShaderError::UnsupportedArgumentTypes {
                            function: function.to_wgsl_for_diagnostics(),
                            arguments: type_names(&types, module),
                        });
                    };
                    for (handle, argument) in handles.iter_mut().zip(&types) {
                        if *argument == naga::TypeInner::Scalar(scalar) {
                            *handle = module.entry_points[binding_builder.entry_point_index]
                                .function
                                .expressions
                                .append(
                                    naga::Expression::Splat {
                                        size,
                                        value: *handle,
                                    },
                                    naga::Span::default(),
                                );
                        }
                    }
                }
                let mut handles = handles.into_iter();
                let arg = handles.next().expect("Math expression without arguments");
                let arg1 = handles.next();
                let arg2 = handles.next();
                let arg3 = handles.next();
                if handles.next().is_some() {
                    panic!(
                        "Math expression with more than 4 arguments is not supported by naga::Expression::Math"
                    );
                }
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::Math {
                            fun: function,
                            arg,
                            arg1,
                            arg2,
                            arg3,
                        },
                        naga::Span::default(),
                    )
            }
            Expression::InstanceField(field) => {
                if !binding_builder.instances.contains_key(&field.buffer_handle) {
                    crate::integrate::integrate_instance(
                        &field.descriptor,
                        &field.inner,
                        &field.buffer_handle,
                        module,
                        binding_builder,
                    );
                }
                let function = &mut module.entry_points[binding_builder.entry_point_index].function;
                let components: Vec<_> = binding_builder.instances[&field.buffer_handle].fields
                    [field.field_index]
                    .function_arguments
                    .iter()
                    .map(|&argument| {
                        function.expressions.append(
                            naga::Expression::FunctionArgument(argument),
                            naga::Span::default(),
                        )
                    })
                    .collect();
                match components[..] {
                    [component] => component,
                    _ => {
                        let ty = module.types.insert(
                            field.descriptor.fields[field.field_index].naga_type.clone(),
                            naga::Span::default(),
                        );
                        function.expressions.append(
                            naga::Expression::Compose { ty, components },
                            naga::Span::default(),
                        )
                    }
                }
            }
            Expression::TextureField(field) => {
                binding_builder
                    .textures
                    .entry(field.handle)
                    .or_insert(crate::TextureBinding {
                        inner: field.inner.clone(),
                    });
                let sampler_type = module.types.insert(
                    naga::Type {
                        name: None,
                        inner: naga::TypeInner::Sampler { comparison: false },
                    },
                    naga::Span::default(),
                );
                let texture_group = binding_builder.current_bind_group;
                binding_builder.current_bind_group += 1;
                let sampler_variable = module.global_variables.append(
                    GlobalVariable {
                        name: Some("u_sampler".to_string()),
                        space: naga::AddressSpace::Handle,
                        binding: Some(ResourceBinding {
                            group: texture_group,
                            binding: 0,
                        }),
                        ty: sampler_type,
                        init: None,
                        memory_decorations: naga::MemoryDecorations::empty(),
                    },
                    Span::default(),
                );
                let texture_type = module.types.insert(
                    naga::Type {
                        name: None,
                        inner: naga::TypeInner::Image {
                            dim: naga::ImageDimension::D2,
                            arrayed: false,
                            class: naga::ImageClass::Sampled {
                                kind: naga::ScalarKind::Float,
                                multi: false,
                            },
                        },
                    },
                    naga::Span::default(),
                );
                let texture = module.global_variables.append(
                    GlobalVariable {
                        name: Some("u_texture".to_string()),
                        space: naga::AddressSpace::Handle,
                        binding: Some(ResourceBinding {
                            group: texture_group,
                            binding: 1,
                        }),
                        ty: texture_type,
                        init: None,
                        memory_decorations: naga::MemoryDecorations::empty(),
                    },
                    Span::default(),
                );
                let image = module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(naga::Expression::GlobalVariable(texture), Span::default());
                let sampler = module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::GlobalVariable(sampler_variable),
                        Span::default(),
                    );
                let coordinate = field.coordinate.setup(module, binding_builder)?;
                let coordinate_type = resolve_type(coordinate, module, binding_builder)?;
                if coordinate_type != vector(naga::VectorSize::Bi) {
                    return Err(ShaderError::UnsupportedArgumentTypes {
                        function: "sample".to_string(),
                        arguments: type_name(&coordinate_type, module),
                    });
                }
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::ImageSample {
                            image,
                            sampler,
                            gather: None,
                            coordinate,
                            array_index: None,
                            offset: None,
                            level: naga::SampleLevel::Auto,
                            depth_ref: None,
                            clamp_to_edge: false,
                        },
                        naga::Span::default(),
                    )
            }
            Expression::UniformField(field) => {
                if !binding_builder.uniforms.contains_key(&field.buffer_handle) {
                    crate::integrate::integrate_uniform(
                        &field.descriptor.borrow(),
                        &field.inner,
                        &field.buffer_handle,
                        module,
                        binding_builder,
                        &field.bind_group_layout,
                    );
                }
                let access_index = module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::AccessIndex {
                            index: field.field_index as u32,
                            base: binding_builder.uniforms[&field.buffer_handle].expression,
                        },
                        naga::Span::default(),
                    );
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::Load {
                            pointer: access_index,
                        },
                        naga::Span::default(),
                    )
            }
            Expression::UV => {
                let function_argument = module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(naga::Expression::FunctionArgument(0), Span::default());

                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::AccessIndex {
                            base: function_argument,
                            index: 2,
                        },
                        Span::default(),
                    )
            }
            Expression::Normal => load_local_variable("_visula_normal", module, binding_builder)?,
            Expression::Position => {
                load_local_variable("_visula_position", module, binding_builder)?
            }
            Expression::ViewDirection => {
                load_local_variable("_visula_view_direction", module, binding_builder)?
            }
            Expression::InputColor => {
                load_local_variable("_visula_input_color", module, binding_builder)?
            }
            Expression::Lit(color) => {
                let color_handle = color.setup(module, binding_builder)?;
                let is_vec4 = is_color_vec4("lit", color_handle, module, binding_builder)?;
                let normal_handle = load_local_variable("_visula_normal", module, binding_builder)?;
                let view_handle =
                    load_local_variable("_visula_view_direction", module, binding_builder)?;
                let position_handle =
                    load_local_variable("_visula_position", module, binding_builder)?;

                let fname = if is_vec4 {
                    "visula_lit_vec4"
                } else {
                    "visula_lit_vec3"
                };
                let function = find_function(module, fname)?;
                let ep = binding_builder.entry_point_index;
                let result = module.entry_points[ep]
                    .function
                    .expressions
                    .append(naga::Expression::CallResult(function), Span::default());
                binding_builder
                    .pending_statements
                    .push(naga::Statement::Call {
                        function,
                        arguments: vec![color_handle, normal_handle, view_handle, position_handle],
                        result: Some(result),
                    });
                result
            }
            Expression::ToonLit(color) => {
                let color_handle = color.setup(module, binding_builder)?;
                let is_vec4 = is_color_vec4("toon_lit", color_handle, module, binding_builder)?;
                let normal_handle = load_local_variable("_visula_normal", module, binding_builder)?;
                let view_handle =
                    load_local_variable("_visula_view_direction", module, binding_builder)?;
                let position_handle =
                    load_local_variable("_visula_position", module, binding_builder)?;

                let fname = if is_vec4 {
                    "visula_toon_lit_vec4"
                } else {
                    "visula_toon_lit_vec3"
                };
                let function = find_function(module, fname)?;
                let ep = binding_builder.entry_point_index;
                let result = module.entry_points[ep]
                    .function
                    .expressions
                    .append(naga::Expression::CallResult(function), Span::default());
                binding_builder
                    .pending_statements
                    .push(naga::Statement::Call {
                        function,
                        arguments: vec![color_handle, normal_handle, view_handle, position_handle],
                        result: Some(result),
                    });
                result
            }
            Expression::DirectionalLit(color) => {
                let color_handle = color.setup(module, binding_builder)?;
                let is_vec4 =
                    is_color_vec4("directional_lit", color_handle, module, binding_builder)?;
                let normal_handle = load_local_variable("_visula_normal", module, binding_builder)?;
                let position_handle =
                    load_local_variable("_visula_position", module, binding_builder)?;

                let fname = if is_vec4 {
                    "visula_directional_lit_vec4"
                } else {
                    "visula_directional_lit_vec3"
                };
                let function = find_function(module, fname)?;
                let ep = binding_builder.entry_point_index;
                let result = module.entry_points[ep]
                    .function
                    .expressions
                    .append(naga::Expression::CallResult(function), Span::default());
                binding_builder
                    .pending_statements
                    .push(naga::Statement::Call {
                        function,
                        arguments: vec![color_handle, normal_handle, position_handle],
                        result: Some(result),
                    });
                result
            }
            Expression::Shared(shared) => {
                if let Some(handle) = binding_builder.shared.get(&shared.id) {
                    return Ok(*handle);
                }
                let handle = shared.value.setup(module, binding_builder)?;
                let function = &mut module.entry_points[binding_builder.entry_point_index].function;
                let expression = &function.expressions[handle];
                let already_available = expression.needs_pre_emit()
                    || matches!(expression, naga::Expression::CallResult(_))
                    || function.named_expressions.contains_key(&handle);
                if !already_available {
                    let name = shared.name.as_deref().unwrap_or("_visula_shared");
                    function.named_expressions.insert(handle, name.to_string());
                    binding_builder
                        .pending_statements
                        .push(naga::Statement::Emit(naga::Range::new_from_bounds(
                            handle, handle,
                        )));
                }
                binding_builder.shared.insert(shared.id, handle);
                handle
            }
        })
    }
}

impl std::fmt::Debug for Expression {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        let value = self.clone();
        match value {
            Expression::BinaryOperator { left, right, .. } => {
                write!(fmt, "BinaryOperator {{ left:")?;
                left.fmt(fmt)?;
                write!(fmt, "right: ")?;
                right.fmt(fmt)?;
                write!(fmt, "}}")?;
            }
            Expression::UnaryOperator { value, .. } => {
                write!(fmt, "UnaryOperator {{ value:")?;
                value.fmt(fmt)?;
                write!(fmt, "}}")?;
            }
            Expression::Literal(v) => {
                write!(fmt, "{v:?}")?;
            }
            Expression::InstanceField(_) => {
                write!(fmt, "InstanceField")?;
            }
            Expression::TextureField(_) => {
                write!(fmt, "TextureField")?;
            }
            Expression::UniformField(_) => {
                write!(fmt, "UniformField")?;
            }
            Expression::Vector2 { .. } => {
                write!(fmt, "Vector2")?;
            }
            Expression::Vector3 { .. } => {
                write!(fmt, "Vector3")?;
            }
            Expression::Vector4 { .. } => {
                write!(fmt, "Vector4")?;
            }
            Expression::Matrix4 { .. } => {
                write!(fmt, "Matrix4")?;
            }
            Expression::Math { function, .. } => {
                write!(fmt, "{function:?}")?;
            }
            Expression::UV => {
                write!(fmt, "UV")?;
            }
            Expression::Normal => {
                write!(fmt, "Normal")?;
            }
            Expression::Position => {
                write!(fmt, "Position")?;
            }
            Expression::InputColor => {
                write!(fmt, "InputColor")?;
            }
            Expression::DirectionalLit(_) => {
                write!(fmt, "DirectionalLit")?;
            }
            Expression::Lit(_) => {
                write!(fmt, "Lit")?;
            }
            Expression::ToonLit(_) => {
                write!(fmt, "ToonLit")?;
            }
            Expression::ViewDirection => {
                write!(fmt, "ViewDirection")?;
            }
            Expression::Shared(shared) => {
                write!(fmt, "Shared {{ value: ")?;
                shared.value.fmt(fmt)?;
                write!(fmt, "}}")?;
            }
        }
        Ok(())
    }
}

impl<T> Div<T> for Expression
where
    T: Into<Expression>,
{
    type Output = Expression;

    fn div(self, other: T) -> Expression {
        let other_expression: Expression = other.into();
        Expression::BinaryOperator {
            left: ExpressionInner::new(self),
            right: ExpressionInner::new(other_expression),
            operator: naga::BinaryOperator::Divide,
        }
    }
}

impl<T> Div<T> for &Expression
where
    Expression: From<T>,
{
    type Output = Expression;

    fn div(self, other: T) -> Expression {
        self.clone() / Expression::from(other)
    }
}

impl Div<Expression> for f32 {
    type Output = Expression;

    fn div(self, other: Expression) -> Expression {
        Expression::from(self) / other
    }
}

impl Div<&Expression> for f32 {
    type Output = Expression;

    fn div(self, other: &Expression) -> Expression {
        Expression::from(self) / other
    }
}

impl Add<Expression> for f32 {
    type Output = Expression;

    fn add(self, other: Expression) -> Expression {
        other + self
    }
}

impl Add<&Expression> for f32 {
    type Output = Expression;

    fn add(self, other: &Expression) -> Expression {
        other + Expression::from(self)
    }
}

impl Add<Expression> for i32 {
    type Output = Expression;

    fn add(self, other: Expression) -> Expression {
        other + self
    }
}

impl Add<&Expression> for i32 {
    type Output = Expression;

    fn add(self, other: &Expression) -> Expression {
        other + Expression::from(self)
    }
}

impl Add<Expression> for glam::Vec2 {
    type Output = Expression;

    fn add(self, other: Expression) -> Expression {
        other + Expression::from(self)
    }
}

impl Add<&Expression> for glam::Vec2 {
    type Output = Expression;

    fn add(self, other: &Expression) -> Expression {
        Expression::from(self) + other
    }
}

impl Add<Expression> for glam::Vec3 {
    type Output = Expression;

    fn add(self, other: Expression) -> Expression {
        other + Expression::from(self)
    }
}

impl Add<&Expression> for glam::Vec3 {
    type Output = Expression;

    fn add(self, other: &Expression) -> Expression {
        Expression::from(self) + other
    }
}

impl<T> Add<T> for Expression
where
    T: Into<Expression>,
{
    type Output = Expression;

    fn add(self, other: T) -> Expression {
        let other_expression: Expression = other.into();
        Expression::BinaryOperator {
            left: ExpressionInner::new(self),
            right: ExpressionInner::new(other_expression),
            operator: naga::BinaryOperator::Add,
        }
    }
}

impl<T> Add<T> for &Expression
where
    Expression: From<T>,
{
    type Output = Expression;

    fn add(self, other: T) -> Expression {
        self.clone() + Expression::from(other)
    }
}

impl Add<Expression> for glam::Vec4 {
    type Output = Expression;

    fn add(self, other: Expression) -> Expression {
        other + Expression::from(self)
    }
}

impl Add<&Expression> for glam::Vec4 {
    type Output = Expression;

    fn add(self, other: &Expression) -> Expression {
        Expression::from(self) + other
    }
}

impl<T> Sub<T> for Expression
where
    T: Into<Expression>,
{
    type Output = Expression;

    fn sub(self, other: T) -> Expression {
        let other_expression: Expression = other.into();
        Expression::BinaryOperator {
            left: ExpressionInner::new(self),
            right: ExpressionInner::new(other_expression),
            operator: naga::BinaryOperator::Subtract,
        }
    }
}

impl<T> Sub<T> for &Expression
where
    Expression: From<T>,
{
    type Output = Expression;

    fn sub(self, other: T) -> Expression {
        self.clone() - Expression::from(other)
    }
}

impl Sub<Expression> for f32 {
    type Output = Expression;

    fn sub(self, other: Expression) -> Expression {
        Expression::from(self) - other
    }
}

impl Sub<&Expression> for f32 {
    type Output = Expression;

    fn sub(self, other: &Expression) -> Expression {
        Expression::from(self) - other
    }
}

impl Sub<Expression> for glam::Vec2 {
    type Output = Expression;

    fn sub(self, other: Expression) -> Expression {
        Expression::from(self) - other
    }
}

impl Sub<&Expression> for glam::Vec2 {
    type Output = Expression;

    fn sub(self, other: &Expression) -> Expression {
        Expression::from(self) - other
    }
}

impl Sub<Expression> for glam::Vec3 {
    type Output = Expression;

    fn sub(self, other: Expression) -> Expression {
        Expression::from(self) - other
    }
}

impl Sub<&Expression> for glam::Vec3 {
    type Output = Expression;

    fn sub(self, other: &Expression) -> Expression {
        Expression::from(self) - other
    }
}

impl Sub<Expression> for glam::Vec4 {
    type Output = Expression;

    fn sub(self, other: Expression) -> Expression {
        Expression::from(self) - other
    }
}

impl Sub<&Expression> for glam::Vec4 {
    type Output = Expression;

    fn sub(self, other: &Expression) -> Expression {
        Expression::from(self) - other
    }
}

impl Neg for Expression {
    type Output = Expression;

    fn neg(self) -> Expression {
        Expression::UnaryOperator {
            value: ExpressionInner::new(self),
            operator: naga::UnaryOperator::Negate,
        }
    }
}

impl Neg for &Expression {
    type Output = Expression;

    fn neg(self) -> Expression {
        -self.clone()
    }
}

impl<T> Mul<T> for Expression
where
    T: Into<Expression>,
{
    type Output = Expression;

    fn mul(self, other: T) -> Expression {
        let other_expression: Expression = other.into();
        Expression::BinaryOperator {
            left: ExpressionInner::new(self),
            right: ExpressionInner::new(other_expression),
            operator: naga::BinaryOperator::Multiply,
        }
    }
}

impl<T> Mul<T> for &Expression
where
    Expression: From<T>,
{
    type Output = Expression;

    fn mul(self, other: T) -> Expression {
        self.clone() * Expression::from(other)
    }
}

impl<T> Rem<T> for Expression
where
    T: Into<Expression>,
{
    type Output = Expression;

    fn rem(self, other: T) -> Expression {
        let other_expression: Expression = other.into();
        Expression::BinaryOperator {
            left: ExpressionInner::new(self),
            right: ExpressionInner::new(other_expression),
            operator: naga::BinaryOperator::Modulo,
        }
    }
}

impl<T> Rem<T> for &Expression
where
    Expression: From<T>,
{
    type Output = Expression;

    fn rem(self, other: T) -> Expression {
        self.clone() % Expression::from(other)
    }
}

impl Mul<Expression> for f32 {
    type Output = Expression;

    fn mul(self, other: Expression) -> Expression {
        Expression::BinaryOperator {
            left: ExpressionInner::new(self.into()),
            right: ExpressionInner::new(other),
            operator: naga::BinaryOperator::Multiply,
        }
    }
}

impl Mul<&Expression> for f32 {
    type Output = Expression;

    fn mul(self, other: &Expression) -> Expression {
        Expression::BinaryOperator {
            left: ExpressionInner::new(self.into()),
            right: ExpressionInner::new(other.into()),
            operator: naga::BinaryOperator::Multiply,
        }
    }
}

impl From<&Expression> for Expression {
    fn from(value: &Expression) -> Expression {
        value.clone()
    }
}

impl From<f32> for Expression {
    fn from(value: f32) -> Expression {
        Expression::Literal(naga::Literal::F32(value))
    }
}

impl From<i32> for Expression {
    fn from(value: i32) -> Expression {
        Expression::Literal(naga::Literal::I32(value))
    }
}

impl From<glam::Vec2> for Expression {
    fn from(value: glam::Vec2) -> Expression {
        Expression::Vector2 {
            x: value.x.into(),
            y: value.y.into(),
        }
    }
}

impl From<glam::Vec3> for Expression {
    fn from(value: glam::Vec3) -> Expression {
        Expression::Vector3 {
            x: value.x.into(),
            y: value.y.into(),
            z: value.z.into(),
        }
    }
}

impl From<glam::Vec4> for Expression {
    fn from(value: glam::Vec4) -> Expression {
        Expression::Vector4 {
            x: value.x.into(),
            y: value.y.into(),
            z: value.z.into(),
            w: value.w.into(),
        }
    }
}

impl From<glam::Quat> for Expression {
    fn from(value: glam::Quat) -> Expression {
        Expression::Vector4 {
            x: value.x.into(),
            y: value.y.into(),
            z: value.z.into(),
            w: value.w.into(),
        }
    }
}

impl From<glam::Mat4> for Expression {
    fn from(value: glam::Mat4) -> Expression {
        mat4(value.x_axis, value.y_axis, value.z_axis, value.w_axis)
    }
}

impl From<[f32; 2]> for Expression {
    fn from(value: [f32; 2]) -> Expression {
        vec2(value[0], value[1])
    }
}

impl From<[f32; 3]> for Expression {
    fn from(value: [f32; 3]) -> Expression {
        vec3(value[0], value[1], value[2])
    }
}

impl From<[f32; 4]> for Expression {
    fn from(value: [f32; 4]) -> Expression {
        vec4(value[0], value[1], value[2], value[3])
    }
}

pub fn vec2(x: impl Into<ExpressionInner>, y: impl Into<ExpressionInner>) -> Expression {
    Expression::Vector2 {
        x: x.into(),
        y: y.into(),
    }
}

pub fn vec3(
    x: impl Into<ExpressionInner>,
    y: impl Into<ExpressionInner>,
    z: impl Into<ExpressionInner>,
) -> Expression {
    Expression::Vector3 {
        x: x.into(),
        y: y.into(),
        z: z.into(),
    }
}

pub fn vec4(
    x: impl Into<ExpressionInner>,
    y: impl Into<ExpressionInner>,
    z: impl Into<ExpressionInner>,
    w: impl Into<ExpressionInner>,
) -> Expression {
    Expression::Vector4 {
        x: x.into(),
        y: y.into(),
        z: z.into(),
        w: w.into(),
    }
}

pub fn mat4(
    x_axis: impl Into<ExpressionInner>,
    y_axis: impl Into<ExpressionInner>,
    z_axis: impl Into<ExpressionInner>,
    w_axis: impl Into<ExpressionInner>,
) -> Expression {
    Expression::Matrix4 {
        x_axis: x_axis.into(),
        y_axis: y_axis.into(),
        z_axis: z_axis.into(),
        w_axis: w_axis.into(),
    }
}

pub fn min(a: impl Into<ExpressionInner>, b: impl Into<ExpressionInner>) -> Expression {
    math(naga::MathFunction::Min, vec![a.into(), b.into()])
}

pub fn max(a: impl Into<ExpressionInner>, b: impl Into<ExpressionInner>) -> Expression {
    math(naga::MathFunction::Max, vec![a.into(), b.into()])
}

pub fn clamp(
    value: impl Into<ExpressionInner>,
    low: impl Into<ExpressionInner>,
    high: impl Into<ExpressionInner>,
) -> Expression {
    math(
        naga::MathFunction::Clamp,
        vec![value.into(), low.into(), high.into()],
    )
}

pub fn mix(
    a: impl Into<ExpressionInner>,
    b: impl Into<ExpressionInner>,
    amount: impl Into<ExpressionInner>,
) -> Expression {
    math(
        naga::MathFunction::Mix,
        vec![a.into(), b.into(), amount.into()],
    )
}

pub fn smoothstep(
    edge_low: impl Into<ExpressionInner>,
    edge_high: impl Into<ExpressionInner>,
    value: impl Into<ExpressionInner>,
) -> Expression {
    math(
        naga::MathFunction::SmoothStep,
        vec![edge_low.into(), edge_high.into(), value.into()],
    )
}

pub fn step(edge: impl Into<ExpressionInner>, value: impl Into<ExpressionInner>) -> Expression {
    math(naga::MathFunction::Step, vec![edge.into(), value.into()])
}

#[cfg(test)]
mod tests {
    use bytemuck::{Pod, Zeroable};
    use naga::ShaderStage;
    use visula_derive::{Instance, Uniform};

    use super::*;
    use crate::test_util::{device, inject_fields, lower_to_module, lower_to_wgsl, write_wgsl};
    use crate::{InstanceBuffer, TextureBuffer, UniformBuffer};

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable, Uniform)]
    struct Settings {
        time: f32,
        scale: f32,
    }

    #[test]
    fn literal() {
        let wgsl = lower_to_wgsl(ShaderStage::Vertex, &[("f32", 1.5.into())]);
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn matrix4() {
        let transform = glam::Mat4::from_translation(glam::Vec3::new(1.0, 2.0, 3.0));
        let wgsl = lower_to_wgsl(ShaderStage::Vertex, &[("mat4x4<f32>", transform.into())]);
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn mat4_from_expressions() {
        let device = device();
        let t = InstanceBuffer::<f32>::new(&device).instance();
        let transform = mat4(
            glam::Vec4::X,
            glam::Vec4::Y,
            glam::Vec4::Z,
            vec4(&t, 0.0, 0.0, 1.0),
        );
        let wgsl = lower_to_wgsl(ShaderStage::Vertex, &[("mat4x4<f32>", transform)]);
        insta::assert_snapshot!(wgsl);
    }

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable, Instance)]
    struct Particle {
        transform: glam::Mat4,
        color: glam::Vec4,
    }

    #[test]
    fn instance_field_mat4() {
        let device = device();
        let particle = InstanceBuffer::<Particle>::new(&device).instance();
        let wgsl = lower_to_wgsl(
            ShaderStage::Vertex,
            &[
                ("mat4x4<f32>", particle.transform),
                ("vec4<f32>", particle.color),
            ],
        );
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn instance_field_vec3() {
        let device = device();
        let t = InstanceBuffer::<f32>::new(&device).instance();
        let wgsl = lower_to_wgsl(
            ShaderStage::Vertex,
            &[("vec3<f32>", vec3(t.cos(), t.sin(), &t))],
        );
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn uniform_field() {
        let device = device();
        let settings = UniformBuffer::<Settings>::new(&device).uniform();
        let wgsl = lower_to_wgsl(ShaderStage::Vertex, &[("f32", settings.time)]);
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn shared_sub_expression() {
        let device = device();
        let t = InstanceBuffer::<f32>::new(&device).instance();
        let s = (&t * 2.0).shared();
        let wgsl = lower_to_wgsl(
            ShaderStage::Vertex,
            &[("vec3<f32>", vec3(s.cos(), s.sin(), &s))],
        );
        assert_eq!(wgsl.matches(" * ").count(), 1);
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn unshared_sub_expression() {
        let device = device();
        let t = InstanceBuffer::<f32>::new(&device).instance();
        let s = &t * 2.0;
        let wgsl = lower_to_wgsl(
            ShaderStage::Vertex,
            &[("vec3<f32>", vec3(s.cos(), s.sin(), &s))],
        );
        assert_eq!(wgsl.matches(" * ").count(), 3);
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn shared_across_fields() {
        let device = device();
        let t = InstanceBuffer::<f32>::new(&device).instance();
        let phase = (&t * 2.0).shared_with_name("phase");
        let wgsl = lower_to_wgsl(
            ShaderStage::Vertex,
            &[("f32", phase.cos()), ("f32", phase.sin())],
        );
        assert_eq!(wgsl.matches(" * ").count(), 1);
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn shared_across_fields_fragment() {
        let device = device();
        let t = InstanceBuffer::<f32>::new(&device).instance();
        let phase = (&t * 2.0).shared_with_name("phase");
        let wgsl = lower_to_wgsl(
            ShaderStage::Fragment,
            &[
                ("vec3<f32>", vec3(phase.cos(), phase.sin(), 0.0)),
                ("f32", phase.clone()),
            ],
        );
        assert_eq!(wgsl.matches(" * ").count(), 1);
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn nested_shared() {
        let device = device();
        let t = InstanceBuffer::<f32>::new(&device).instance();
        let a = (&t * 2.0).shared();
        let b = (a.cos() + &a).shared();
        let wgsl = lower_to_wgsl(
            ShaderStage::Vertex,
            &[("vec3<f32>", vec3(b.sin(), b.cos(), &a))],
        );
        assert_eq!(wgsl.matches(" * ").count(), 1);
        assert_eq!(wgsl.matches(" + ").count(), 1);
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn shared_without_new_let() {
        let device = device();
        let t = InstanceBuffer::<f32>::new(&device).instance();
        let s = (&t * 2.0).shared();
        let wgsl = lower_to_wgsl(
            ShaderStage::Vertex,
            &[("vec3<f32>", vec3(t.shared(), &s, s.shared()))],
        );
        assert_eq!(wgsl.matches("let ").count(), 2);
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn missing_variable() {
        let error =
            lower_to_module(ShaderStage::Vertex, &[("vec3<f32>", Expression::Normal)]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "variable '_visula_normal' not found in shader"
        );
    }

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable, Uniform)]
    struct Background {
        color: glam::Vec3,
        _padding: f32,
    }

    #[test]
    fn scalar_minus_vec3() {
        let device = device();
        let color = UniformBuffer::<Background>::new(&device).uniform().color;
        let wgsl = lower_to_wgsl(
            ShaderStage::Vertex,
            &[("vec3<f32>", mix(&color, 1.0 - &color, 0.15))],
        );
        insta::assert_snapshot!(wgsl);
    }

    #[test]
    fn scalar_and_vector_operands() {
        let device = device();
        let color = UniformBuffer::<Background>::new(&device).uniform().color;
        let wgsl = lower_to_wgsl(
            ShaderStage::Vertex,
            &[
                ("vec3<f32>", 0.5 + &color),
                ("vec3<f32>", &color - 0.5),
                ("vec3<f32>", 2.0 * &color),
                ("vec3<f32>", 2.0 / &color),
                ("vec3<f32>", &color % 2.0),
            ],
        );
        insta::assert_snapshot!(wgsl);
    }

    fn lowering_error(wgsl_type: &str, expression: Expression) -> String {
        lower_to_module(ShaderStage::Vertex, &[(wgsl_type, expression)])
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn mismatched_operands() {
        let device = device();
        let color = UniformBuffer::<Background>::new(&device).uniform().color;
        let difference = Expression::from(glam::Vec2::ONE) - &color;
        assert_eq!(
            lowering_error("vec3<f32>", mix(&color, difference, 0.15)),
            "unsupported operand types: vec2<f32> - vec3<f32>"
        );
        assert_eq!(
            lowering_error("vec3<f32>", &color + 1),
            "unsupported operand types: vec3<f32> + i32"
        );
        assert_eq!(
            lowering_error("vec3<f32>", Expression::from(glam::Mat4::IDENTITY) * &color),
            "unsupported operand types: mat4x4<f32> * vec3<f32>"
        );
    }

    #[test]
    fn negated_matrix() {
        assert_eq!(
            lowering_error("mat4x4<f32>", -Expression::from(glam::Mat4::IDENTITY)),
            "unsupported operand type: -mat4x4<f32>"
        );
    }

    #[test]
    fn mismatched_math_arguments() {
        let device = device();
        let color = UniformBuffer::<Background>::new(&device).uniform().color;
        assert_eq!(
            lowering_error("f32", color.dot(glam::Vec2::ONE)),
            "unsupported argument types: dot(vec3<f32>, vec2<f32>)"
        );
        assert_eq!(
            lowering_error("vec3<f32>", color.max(1)),
            "unsupported argument types: max(vec3<f32>, i32)"
        );
    }

    #[test]
    fn mismatched_vector_components() {
        assert_eq!(
            lowering_error("vec3<f32>", vec3(1.0, 2, 3.0)),
            "unsupported argument types: vec3<f32>(f32, i32, f32)"
        );
    }

    #[test]
    fn mismatched_texture_coordinate() {
        let device = device();
        let texture = TextureBuffer::<f32>::new(
            &device,
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        assert_eq!(
            lowering_error("vec4<f32>", texture.sample(&glam::Vec3::ONE.into())),
            "unsupported argument types: sample(vec3<f32>)"
        );
    }

    #[test]
    fn lit_vec4_instance_field() {
        let device = device();
        let particle = InstanceBuffer::<Particle>::new(&device).instance();
        let source = format!(
            "{}
struct Fields {{
    a: vec4<f32>,
}}

@fragment
fn fs_main() -> @location(0) vec4<f32> {{
    var _visula_normal: vec3<f32>;
    var _visula_view_direction: vec3<f32>;
    var _visula_position: vec3<f32>;
    _visula_normal = vec3<f32>(0.0, 0.0, 1.0);
    _visula_view_direction = vec3<f32>(0.0, 0.0, 1.0);
    _visula_position = vec3<f32>(0.0);
    var fields: Fields;
    let output = fields;
    return vec4<f32>(0.0);
}}
",
            crate::LIGHTING_WGSL
        );
        let module =
            inject_fields(&source, ShaderStage::Fragment, &[particle.color.lit()]).unwrap();
        assert!(write_wgsl(&module).contains(" = visula_lit_vec4_(color, "));
    }

    #[test]
    fn lit_scalar() {
        assert_eq!(
            lowering_error("vec3<f32>", Expression::from(0.5).lit()),
            "unsupported argument types: lit(f32)"
        );
    }

    #[test]
    fn mismatched_field_type() {
        assert_eq!(
            lowering_error("vec3<f32>", Expression::from(0.5)),
            "mismatched types for fields.a: expected vec3<f32>, found f32"
        );
    }

    #[test]
    fn scalar_math_arguments() {
        let device = device();
        let color = UniformBuffer::<Background>::new(&device).uniform().color;
        let wgsl = lower_to_wgsl(
            ShaderStage::Vertex,
            &[
                ("vec3<f32>", color.max(0.0031308)),
                ("vec3<f32>", color.pow(1.0 / 2.4)),
                ("vec3<f32>", color.step(0.04045)),
                ("vec3<f32>", color.clamp(0.0, 1.0)),
                ("vec3<f32>", color.mix(glam::Vec3::ONE, 0.5)),
            ],
        );
        insta::assert_snapshot!(wgsl);
    }
}
