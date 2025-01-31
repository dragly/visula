use std::fmt::Debug;

use egui::Context;
use winit::event::{Event, WindowEvent};
use winit::window::WindowId;

use crate::application::Application;
use crate::camera::Camera;
use crate::CustomEvent;

pub struct RenderData<'a> {
    pub view: &'a wgpu::TextureView,
    pub multisampled_framebuffer: &'a wgpu::TextureView,
    pub depth_texture: &'a wgpu::TextureView,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub camera: &'a Camera,
}

pub trait Simulation {
    type Error: Debug;
    fn handle_event(&mut self, _application: &mut Application, _event: &Event<CustomEvent>) {}
    fn update(&mut self, _application: &mut Application) {}
    fn render(&mut self, _data: &mut RenderData) {}
    fn gui(&mut self, _application: &Application, _context: &Context) {}
    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        }
    }
}

impl<E> Simulation for Box<dyn Simulation<Error = E>>
where
    E: Debug,
{
    type Error = E;
    fn handle_event(&mut self, application: &mut Application, event: &Event<CustomEvent>) {
        self.as_mut().handle_event(application, event)
    }
    fn update(&mut self, application: &mut Application) {
        self.as_mut().update(application)
    }
    fn render(&mut self, data: &mut RenderData) {
        self.as_mut().render(data)
    }
    fn gui(&mut self, application: &Application, context: &Context) {
        self.as_mut().gui(application, context)
    }
    fn clear_color(&self) -> wgpu::Color {
        self.as_ref().clear_color()
    }
}
