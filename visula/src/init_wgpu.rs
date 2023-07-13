use crate::camera::controller::CameraController;
use crate::{application::Application, camera::Camera};

use crate::custom_event::CustomEvent;
use egui::FontDefinitions;
use egui_wgpu_backend::RenderPass;
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::InstanceDescriptor;

use winit::{event_loop::EventLoopProxy, window::Window};

pub async fn init(proxy: EventLoopProxy<CustomEvent>, window: Window) {
    let event_result =
        proxy.send_event(CustomEvent::Ready(Box::new(Application::new(window).await)));
    if event_result.is_err() {
        println!("ERROR: Could not send event! Is the event loop closed?")
    }
}
