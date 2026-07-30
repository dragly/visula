use std::{cell::RefCell, rc::Rc};

use crate::{
    instance_buffer::InstanceBufferInner,
    integrate::{InstanceDescriptor, InstanceFieldDescriptor},
    naga_type::NagaType,
    value::Expression,
    vertex_attr_format::VertexAttrFormat,
};

pub trait InstanceHandle {}

pub trait Instance {
    type Type: InstanceHandle;
    fn instance(inner: Rc<RefCell<InstanceBufferInner>>) -> Self::Type;
}

#[derive(Clone)]
pub struct InstanceField {
    pub buffer_handle: uuid::Uuid,
    pub field_index: usize,
    pub inner: Rc<RefCell<InstanceBufferInner>>,
    pub descriptor: Rc<InstanceDescriptor>,
}

impl InstanceHandle for Expression {}

macro_rules! impl_instance_for_plain {
    ($($type:ty),* $(,)?) => {
        $(
            impl Instance for $type {
                type Type = Expression;
                fn instance(inner: Rc<RefCell<InstanceBufferInner>>) -> Expression {
                    let descriptor = Rc::new(InstanceDescriptor {
                        struct_size: std::mem::size_of::<$type>() as u64,
                        fields: vec![InstanceFieldDescriptor {
                            name: "value".to_string(),
                            naga_type: <$type as NagaType>::naga_type(),
                            vertex_attr_format: <$type as VertexAttrFormat>::vertex_attr_format(),
                        }],
                    });
                    Expression::InstanceField(InstanceField {
                        buffer_handle: inner.borrow().handle,
                        inner: inner.clone(),
                        field_index: 0,
                        descriptor,
                    })
                }
            }
        )*
    };
}

impl_instance_for_plain!(
    f32,
    [f32; 2],
    [f32; 3],
    [f32; 4],
    glam::Vec2,
    glam::Vec3,
    glam::Vec4,
);
