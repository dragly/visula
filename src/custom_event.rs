use crate::drop_event::DropEvent;
use crate::application::Application;

pub enum CustomEvent {
    Ready(Application),
    DropEvent(DropEvent),
}
