use winit::event::WindowEvent;

use crate::application::Application;

pub trait Simulation: Sized {
    fn init(application: &mut Application) -> Self;
    fn handle_event(&mut self, _application: &mut Application, _event: &WindowEvent) {}
    fn update(&mut self, _application: &Application) {}
    fn render<'a>(&'a mut self, _render_pass: &mut wgpu::RenderPass<'a>) {}
}
