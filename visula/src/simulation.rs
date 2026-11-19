use egui::Context;
use winit::event::Event;

use crate::application::Application;
use crate::camera::Camera;
use crate::light::DirectionalLight;
use crate::pipelines::{Circles, Cylinders, Lines, Polygons, Rects, Renderable, Spheres, Torus};
use crate::post_process::transparency::TransparencyPass;
use crate::CustomEvent;

pub struct PrepareData<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub camera: &'a Camera,
    pub light: &'a DirectionalLight,
}

pub struct RenderData<'a> {
    pub pass: &'a mut wgpu::RenderPass<'static>,
    pub camera: &'a Camera,
    pub light: &'a DirectionalLight,
    pub(crate) transparency: TransparencyRenderData<'a>,
}

impl RenderData<'_> {
    pub fn transparent_pass(&mut self) -> &mut wgpu::RenderPass<'static> {
        self.transparency.pass()
    }
}

pub(crate) struct TransparencyRenderData<'a> {
    encoder: &'a mut wgpu::CommandEncoder,
    transparency: &'a TransparencyPass,
    depth_texture: &'a wgpu::TextureView,
    pass: Option<wgpu::RenderPass<'static>>,
}

impl<'a> TransparencyRenderData<'a> {
    pub(crate) fn new(
        encoder: &'a mut wgpu::CommandEncoder,
        transparency: &'a TransparencyPass,
        depth_texture: &'a wgpu::TextureView,
    ) -> Self {
        Self {
            encoder,
            transparency,
            depth_texture,
            pass: None,
        }
    }

    fn pass(&mut self) -> &mut wgpu::RenderPass<'static> {
        let Self {
            encoder,
            transparency,
            depth_texture,
            pass,
        } = self;
        pass.get_or_insert_with(|| {
            transparency
                .begin_render_pass(encoder, depth_texture)
                .forget_lifetime()
        })
    }

    pub(crate) fn used(&self) -> bool {
        self.pass.is_some()
    }
}

pub struct ShadowRenderData<'a> {
    pub pass: &'a mut wgpu::RenderPass<'static>,
    pub light: &'a DirectionalLight,
}

pub trait Simulation {
    fn handle_event(&mut self, _application: &mut Application, _event: &Event<CustomEvent>) {}
    fn update(&mut self, _application: &mut Application) {}
    fn prepare(&mut self, _data: &mut PrepareData) {}
    fn render(&mut self, _data: &mut RenderData) {}
    fn render_shadow(&mut self, _data: &mut ShadowRenderData) {}
    fn gui(&mut self, _application: &Application, _context: &Context) {}
    /// The background color in linear RGB. Convert hex colors with [`crate::srgb`].
    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        }
    }
}

macro_rules! impl_simulation_for_renderable {
    ($($type:ty),* $(,)?) => {
        $(
            impl Simulation for $type {
                fn render(&mut self, data: &mut RenderData) {
                    Renderable::render(self, data)
                }
                fn render_shadow(&mut self, data: &mut ShadowRenderData) {
                    Renderable::render_shadow(self, data)
                }
            }
        )*
    };
}

impl_simulation_for_renderable!(Circles, Cylinders, Lines, Polygons, Rects, Spheres, Torus);

impl Simulation for Vec<Box<dyn Renderable>> {
    fn render(&mut self, data: &mut RenderData) {
        for renderable in self.iter() {
            renderable.render(data);
        }
    }
    fn render_shadow(&mut self, data: &mut ShadowRenderData) {
        for renderable in self.iter() {
            renderable.render_shadow(data);
        }
    }
}

impl Simulation for Box<dyn Simulation> {
    fn handle_event(&mut self, application: &mut Application, event: &Event<CustomEvent>) {
        self.as_mut().handle_event(application, event)
    }
    fn update(&mut self, application: &mut Application) {
        self.as_mut().update(application)
    }
    fn prepare(&mut self, data: &mut PrepareData) {
        self.as_mut().prepare(data)
    }
    fn render(&mut self, data: &mut RenderData) {
        self.as_mut().render(data)
    }
    fn render_shadow(&mut self, data: &mut ShadowRenderData) {
        self.as_mut().render_shadow(data)
    }
    fn gui(&mut self, application: &Application, context: &Context) {
        self.as_mut().gui(application, context)
    }
    fn clear_color(&self) -> wgpu::Color {
        self.as_ref().clear_color()
    }
}
