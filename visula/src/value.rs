use std::{
    fmt::{Error, Formatter},
    ops::{Add, Div},
};

use crate::{BindingBuilder, InstanceField, UniformField};

#[derive(Clone)]
pub enum ExpressionInner {
    BinaryOperator {
        left: Expression,
        right: Expression,
        operator: naga::BinaryOperator,
    },
    Constant(naga::ConstantInner),
    InstanceField(InstanceField),
    UniformField(UniformField),
    Vector {
        components: Vec<Expression>,
    },
}

pub trait AsValue {
    fn as_value(&self) -> ExpressionInner;
}

impl AsValue for InstanceField {
    fn as_value(&self) -> ExpressionInner {
        ExpressionInner::InstanceField(self.clone())
    }
}

impl AsValue for UniformField {
    fn as_value(&self) -> ExpressionInner {
        ExpressionInner::UniformField(self.clone())
    }
}

#[derive(Clone)]
pub struct Expression {
    inner: Box<ExpressionInner>,
}

impl Expression {
    pub fn setup(
        &self,
        module: &mut naga::Module,
        binding_builder: &mut BindingBuilder,
    ) -> naga::Handle<naga::Expression> {
        let val = self.inner.clone();

        match *val {
            ExpressionInner::Constant(inner) => {
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
            ExpressionInner::Vector { components } => {
                let naga_type = ::naga::Type {
                    name: None,
                    inner: ::naga::TypeInner::Vector {
                        kind: ::naga::ScalarKind::Float,
                        width: 4,
                        size: ::naga::VectorSize::Tri,
                    },
                };
                let field_type = module.types.insert(naga_type, ::naga::Span::default());
                let components_setup = components
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
            ExpressionInner::BinaryOperator {
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
            ExpressionInner::InstanceField(field) => {
                if !binding_builder.bindings.contains_key(&field.buffer_handle) {
                    (field.integrate_buffer)(
                        &field.inner,
                        field.buffer_handle,
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
            ExpressionInner::UniformField(field) => {
                let inner = field.inner.borrow();
                if !binding_builder.bindings.contains_key(&field.buffer_handle) {
                    (field.integrate_buffer)(
                        &field.inner,
                        field.buffer_handle,
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

    pub fn new(value: ExpressionInner) -> Expression {
        Expression {
            inner: Box::new(value),
        }
    }
}

impl std::fmt::Debug for Expression {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        let value = self.inner.clone();
        match *value {
            ExpressionInner::BinaryOperator { left, right, .. } => {
                write!(fmt, "BinaryOperator {{ left:")?;
                left.fmt(fmt)?;
                write!(fmt, "right: ")?;
                right.fmt(fmt)?;
                write!(fmt, "}}")?;
            }
            ExpressionInner::Constant { .. } => {
                write!(fmt, "Constant")?;
            }
            ExpressionInner::InstanceField(_) => {
                write!(fmt, "InstanceField")?;
            }
            ExpressionInner::UniformField(_) => {
                write!(fmt, "UniformField")?;
            }
            ExpressionInner::Vector { .. } => {
                write!(fmt, "Vector")?;
            }
        }
        Ok(())
    }
}

impl Add for &Expression {
    type Output = Expression;

    fn add(self, other: &Expression) -> Expression {
        Expression::new(ExpressionInner::BinaryOperator {
            left: self.clone(),
            right: other.clone(),
            operator: naga::BinaryOperator::Add,
        })
    }
}

impl Add for Expression {
    type Output = Expression;

    fn add(self, other: Expression) -> Expression {
        Expression::new(ExpressionInner::BinaryOperator {
            left: self,
            right: other,
            operator: naga::BinaryOperator::Add,
        })
    }
}

impl Add<f32> for Expression {
    type Output = Expression;

    fn add(self, other: f32) -> Expression {
        let other_scalar: Expression = other.into();
        Expression::new(ExpressionInner::BinaryOperator {
            left: self,
            right: other_scalar,
            operator: naga::BinaryOperator::Add,
        })
    }
}

impl Add<f32> for &Expression {
    type Output = Expression;

    fn add(self, other: f32) -> Expression {
        self.clone() + other
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
        other + self
    }
}

impl Add<&Expression> for Expression {
    type Output = Expression;

    fn add(self, other: &Expression) -> Expression {
        Expression::new(ExpressionInner::BinaryOperator {
            left: self,
            right: other.clone(),
            operator: naga::BinaryOperator::Add,
        })
    }
}

impl Div<f32> for Expression {
    type Output = Expression;

    fn div(self, other: f32) -> Expression {
        let other_scalar: Expression = other.into();
        Expression::new(ExpressionInner::BinaryOperator {
            left: self,
            right: other_scalar,
            operator: naga::BinaryOperator::Divide,
        })
    }
}

impl Div<f32> for &Expression {
    type Output = Expression;

    fn div(self, other: f32) -> Expression {
        self.clone() / other
    }
}

impl Add<i32> for Expression {
    type Output = Expression;

    fn add(self, other: i32) -> Expression {
        let other_scalar: Expression = other.into();
        Expression::new(ExpressionInner::BinaryOperator {
            left: self,
            right: other_scalar,
            operator: naga::BinaryOperator::Add,
        })
    }
}

impl Add<i32> for &Expression {
    type Output = Expression;

    fn add(self, other: i32) -> Expression {
        self.clone() + other
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
        other + self
    }
}
impl From<f32> for Expression {
    fn from(value: f32) -> Expression {
        Expression::new(ExpressionInner::Constant(naga::ConstantInner::Scalar {
            value: naga::ScalarValue::Float(value as f64),
            width: 4,
        }))
    }
}

impl From<i32> for Expression {
    fn from(value: i32) -> Expression {
        Expression::new(ExpressionInner::Constant(naga::ConstantInner::Scalar {
            value: naga::ScalarValue::Sint(value as i64),
            width: 4,
        }))
    }
}
