use winit::{event_loop::EventLoopProxy, window::Window};

use crate::custom_event::CustomEvent;
use crate::init_wgpu::init;

#[cfg(not(target_arch = "wasm32"))]
pub fn setup_other(window: Window, proxy: EventLoopProxy<CustomEvent>) {
    env_logger::init();
    // Temporarily avoid srgb formats for the swapchain on the web
    futures::executor::block_on(init(proxy, window, wgpu::TextureFormat::Bgra8UnormSrgb));
}
