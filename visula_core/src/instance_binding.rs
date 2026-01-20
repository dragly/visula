use std::{cell::RefCell, rc::Rc};

use crate::{instance_buffer::InstanceBufferInner, BindingBuilder};

pub trait InstanceHandle {}

pub trait Instance {
    type Type: InstanceHandle;
    fn instance(inner: Rc<RefCell<InstanceBufferInner>>) -> Self::Type;
}

type IntegrateInstance =
    fn(&Rc<RefCell<InstanceBufferInner>>, &uuid::Uuid, &mut naga::Module, &mut BindingBuilder);

#[derive(Clone)]
pub struct InstanceField {
    pub buffer_handle: uuid::Uuid,
    pub field_index: usize,
    pub inner: Rc<RefCell<InstanceBufferInner>>,
    pub integrate_instance: IntegrateInstance,
}
