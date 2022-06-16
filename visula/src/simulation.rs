use std::fmt::Debug;

use egui::Context;
use winit::event::WindowEvent;

use crate::application::Application;

pub struct SimulationRenderInfo<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub view: &'a wgpu::TextureView,
    pub depth: &'a wgpu::TextureView,
    pub camera_bind_group: &'a wgpu::BindGroup,
}

pub trait Simulation: Sized {
    type Error: Debug;
    fn init(application: &mut Application) -> Result<Self, Self::Error>;
    fn handle_event(&mut self, _application: &mut Application, _event: &WindowEvent) {}
    fn update(&mut self, _application: &Application) {}
    fn render(&mut self, info: &mut SimulationRenderInfo) {}
    fn gui(&mut self, _context: &Context) {}
}
