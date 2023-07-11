use std::{
    fmt::{Error, Formatter},
    ops::{Add, Deref, Div, Mul, Neg, Sub},
};

use crate::{BindingBuilder, InstanceField, UniformField};

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
    Constant(naga::ConstantInner),
    InstanceField(InstanceField),
    UniformField(UniformField),
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
    Length(ExpressionInner),
    Exp(ExpressionInner),
    Pow {
        base: ExpressionInner,
        exponent: ExpressionInner,
    },
}

impl Expression {
    pub fn pow(&self, exponent: impl Into<ExpressionInner>) -> Expression {
        fn inner(base: ExpressionInner, exponent: ExpressionInner) -> Expression {
            Expression::Pow { base, exponent }
        }
        inner(self.into(), exponent.into())
    }

    pub fn exp(&self) -> Expression {
        Expression::Exp(self.into())
    }

    pub fn length(&self) -> Expression {
        Expression::Length(self.into())
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

impl Expression {
    pub fn setup(
        &self,
        module: &mut naga::Module,
        binding_builder: &mut BindingBuilder,
    ) -> naga::Handle<naga::Expression> {
        let val = self.clone();

        match val {
            Expression::Constant(inner) => {
                // TODO handle non-float type
                let constant = module.constants.append(
                    ::naga::Constant {
                        name: None,
                        specialization: None,
                        inner,
                    },
                    ::naga::Span::default(),
                );
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        ::naga::Expression::Constant(constant),
                        ::naga::Span::default(),
                    )
            }
            Expression::Vector2 { x, y } => {
                let naga_type = ::naga::Type {
                    name: None,
                    inner: ::naga::TypeInner::Vector {
                        kind: ::naga::ScalarKind::Float,
                        width: 4,
                        size: ::naga::VectorSize::Bi,
                    },
                };
                let field_type = module.types.insert(naga_type, ::naga::Span::default());
                let components_setup = vec![x, y]
                    .iter()
                    .map(|component| component.setup(module, binding_builder))
                    .collect();
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        ::naga::Expression::Compose {
                            ty: field_type,
                            components: components_setup,
                        },
                        ::naga::Span::default(),
                    )
            }
            Expression::Vector3 { x, y, z } => {
                let naga_type = ::naga::Type {
                    name: None,
                    inner: ::naga::TypeInner::Vector {
                        kind: ::naga::ScalarKind::Float,
                        width: 4,
                        size: ::naga::VectorSize::Tri,
                    },
                };
                let field_type = module.types.insert(naga_type, ::naga::Span::default());
                let components_setup = vec![x, y, z]
                    .iter()
                    .map(|component| component.setup(module, binding_builder))
                    .collect();
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        ::naga::Expression::Compose {
                            ty: field_type,
                            components: components_setup,
                        },
                        ::naga::Span::default(),
                    )
            }
            Expression::Vector4 { x, y, z, w } => {
                let naga_type = ::naga::Type {
                    name: None,
                    inner: ::naga::TypeInner::Vector {
                        kind: ::naga::ScalarKind::Float,
                        width: 4,
                        size: ::naga::VectorSize::Quad,
                    },
                };
                let field_type = module.types.insert(naga_type, ::naga::Span::default());
                let components_setup = vec![x, y, z, w]
                    .iter()
                    .map(|component| component.setup(module, binding_builder))
                    .collect();
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        ::naga::Expression::Compose {
                            ty: field_type,
                            components: components_setup,
                        },
                        ::naga::Span::default(),
                    )
            }
            Expression::BinaryOperator {
                left,
                right,
                operator,
            } => {
                let left_setup = left.setup(module, binding_builder);
                let right_setup = right.setup(module, binding_builder);
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
                let value_setup = value.setup(module, binding_builder);
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
            Expression::Length(value) => {
                let arg = value.setup(module, binding_builder);
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::Math {
                            fun: naga::MathFunction::Length,
                            arg,
                            arg1: None,
                            arg2: None,
                            arg3: None,
                        },
                        naga::Span::default(),
                    )
            }
            Expression::Exp(value) => {
                let arg = value.setup(module, binding_builder);
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::Math {
                            fun: naga::MathFunction::Exp,
                            arg,
                            arg1: None,
                            arg2: None,
                            arg3: None,
                        },
                        naga::Span::default(),
                    )
            }
            Expression::Pow { base, exponent } => {
                let arg = base.setup(module, binding_builder);
                let arg1 = Some(exponent.setup(module, binding_builder));
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::Math {
                            fun: naga::MathFunction::Pow,
                            arg,
                            arg1,
                            arg2: None,
                            arg3: None,
                        },
                        naga::Span::default(),
                    )
            }
            Expression::InstanceField(field) => {
                if !binding_builder.bindings.contains_key(&field.buffer_handle) {
                    (field.integrate_buffer)(
                        &field.inner,
                        &field.buffer_handle,
                        module,
                        binding_builder,
                    );
                }
                module.entry_points[binding_builder.entry_point_index]
                    .function
                    .expressions
                    .append(
                        naga::Expression::FunctionArgument(
                            binding_builder.bindings[&field.buffer_handle].fields
                                [field.field_index]
                                .function_argument,
                        ),
                        naga::Span::default(),
                    )
            }
            Expression::UniformField(field) => {
                let inner = field.inner.borrow();
                if !binding_builder.bindings.contains_key(&field.buffer_handle) {
                    (field.integrate_buffer)(
                        &field.inner,
                        &field.buffer_handle,
                        module,
                        binding_builder,
                        &inner.bind_group_layout,
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
        }
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
            Expression::Constant { .. } => {
                write!(fmt, "Constant")?;
            }
            Expression::InstanceField(_) => {
                write!(fmt, "InstanceField")?;
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
            Expression::Length(_) => {
                write!(fmt, "Length")?;
            }
            Expression::Exp(_) => {
                write!(fmt, "Exp")?;
            }
            Expression::Pow { .. } => {
                write!(fmt, "Pow")?;
            }
        }
        Ok(())
    }
}

impl Div<f32> for Expression {
    type Output = Expression;

    fn div(self, other: f32) -> Expression {
        let other_scalar: Expression = other.into();
        Expression::BinaryOperator {
            left: ExpressionInner::new(self),
            right: ExpressionInner::new(other_scalar),
            operator: naga::BinaryOperator::Divide,
        }
    }
}

impl Div<f32> for &Expression {
    type Output = Expression;

    fn div(self, other: f32) -> Expression {
        self.clone() / other
    }
}

impl Div<Expression> for Expression {
    type Output = Expression;

    fn div(self, other: Expression) -> Expression {
        Expression::BinaryOperator {
            left: ExpressionInner::new(self),
            right: ExpressionInner::new(other),
            operator: naga::BinaryOperator::Divide,
        }
    }
}

impl Div<Expression> for &Expression {
    type Output = Expression;

    fn div(self, other: Expression) -> Expression {
        self.clone() / other
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

impl Sub<Expression> for Expression {
    type Output = Expression;

    fn sub(self, other: Expression) -> Expression {
        Expression::BinaryOperator {
            left: ExpressionInner::new(self),
            right: ExpressionInner::new(other),
            operator: naga::BinaryOperator::Subtract,
        }
    }
}

impl Sub<&Expression> for Expression {
    type Output = Expression;

    fn sub(self, other: &Expression) -> Expression {
        self - other.clone()
    }
}

impl Sub<&Expression> for &Expression {
    type Output = Expression;

    fn sub(self, other: &Expression) -> Expression {
        self.clone() - other.clone()
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

impl Mul<Expression> for Expression {
    type Output = Expression;

    fn mul(self, other: Expression) -> Expression {
        Expression::BinaryOperator {
            left: ExpressionInner::new(self),
            right: ExpressionInner::new(other),
            operator: naga::BinaryOperator::Multiply,
        }
    }
}

impl Mul<&Expression> for Expression {
    type Output = Expression;

    fn mul(self, other: &Expression) -> Expression {
        self * other.clone()
    }
}

impl Mul<&Expression> for &Expression {
    type Output = Expression;

    fn mul(self, other: &Expression) -> Expression {
        self.clone() * other.clone()
    }
}

impl Mul<f32> for Expression {
    type Output = Expression;

    fn mul(self, other: f32) -> Expression {
        Expression::BinaryOperator {
            left: ExpressionInner::new(self),
            right: ExpressionInner::new(other.into()),
            operator: naga::BinaryOperator::Multiply,
        }
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
        Expression::Constant(naga::ConstantInner::Scalar {
            value: naga::ScalarValue::Float(value as f64),
            width: 4,
        })
    }
}

impl From<i32> for Expression {
    fn from(value: i32) -> Expression {
        Expression::Constant(naga::ConstantInner::Scalar {
            value: naga::ScalarValue::Sint(value as i64),
            width: 4,
        })
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
