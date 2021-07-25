use std::fmt::Debug;

use egui::Context;
use winit::event::Event;

use crate::application::Application;
use crate::CustomEvent;

pub struct SimulationRenderData<'a> {
    pub view: &'a wgpu::TextureView,
    pub depth_texture: &'a wgpu::TextureView,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub camera_bind_group: &'a wgpu::BindGroup,
}

pub trait Simulation: Sized {
    type Error: Debug;
    fn init(application: &mut Application) -> Result<Self, Self::Error>;
    fn handle_event(&mut self, _application: &mut Application, _event: &Event<CustomEvent>) {}
    fn update(&mut self, _application: &Application) {}
    fn render(&mut self, _data: &mut SimulationRenderData) {}
    fn gui(&mut self, _context: &Context) {}
    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        }
    }
}
