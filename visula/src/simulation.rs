use winit::event::Event;

use crate::{CustomEvent, application::Application};

pub trait Simulation: Sized {
    fn init(application: &mut Application) -> Self;
    fn handle_event(&mut self, _application: &mut Application, _event: &Event<CustomEvent>) {}
    fn update(&mut self, _application: &Application) {}
    fn render<'a>(&'a mut self, _render_pass: &mut wgpu::RenderPass<'a>) {}
}
