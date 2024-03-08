use crate::drop_event::DropEvent;

#[derive(Debug)]
pub enum CustomEvent {
    DropEvent(DropEvent),
}
