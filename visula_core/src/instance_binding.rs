use std::{cell::RefCell, rc::Rc};

use crate::{instance_buffer::InstanceBufferInner, integrate::InstanceDescriptor};

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
