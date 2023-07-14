
use crate::{application::Application};

use crate::custom_event::CustomEvent;





use winit::{event_loop::EventLoopProxy, window::Window};

pub async fn init(proxy: EventLoopProxy<CustomEvent>, window: Window) {
    let event_result =
        proxy.send_event(CustomEvent::Ready(Box::new(Application::new(window).await)));
    if event_result.is_err() {
        println!("ERROR: Could not send event! Is the event loop closed?")
    }
}
