use crate::{drop_event::DropEvent, Application};

#[derive(Debug)]
pub enum CustomEvent {
    Application(Application),
    DropEvent(DropEvent),
}
