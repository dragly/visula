use std::{cell::RefCell, rc::Rc};

use crate::{texture_buffer::TextureBufferInner, Expression};

pub trait TextureHandle {}

pub trait Texture {
    type Type: TextureHandle;
    fn texture(inner: Rc<RefCell<TextureBufferInner>>) -> Self::Type;
}

#[derive(Clone)]
pub struct TextureField {
    pub handle: uuid::Uuid,
    pub inner: Rc<RefCell<TextureBufferInner>>,
    pub coordinate: Box<Expression>,
}
