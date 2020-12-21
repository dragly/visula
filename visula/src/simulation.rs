use winit::event::WindowEvent;

use crate::application::Application;

pub trait Simulation: Sized {
    fn init(application: &mut Application) -> Self;
    fn handle_event(&mut self, application: &mut Application, event: &WindowEvent);
    fn update(&mut self, application: &Application);
    fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>);
}
