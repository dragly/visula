use crate::application::Application;
//#[cfg(target_arch = "wasm32")]
use crate::drop_event::DropEvent;

pub enum CustomEvent {
    Ready(Box<Application>),
    DropEvent(DropEvent),
}
