use crate::application::Application;
//#[cfg(target_arch = "wasm32")]
use crate::drop_event::DropEvent;

pub enum CustomEvent {
    Ready(Application),
    //#[cfg(target_arch = "wasm32")]
    DropEvent(DropEvent),
}
