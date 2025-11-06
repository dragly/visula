use egui::Slider;

use egui_wgpu::ScreenDescriptor;
use std::time::Duration;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopProxy;

use pyo3::prelude::*;
use pyo3::types::PyFunction;

use visula::{create_application, create_window, Application, CustomEvent, RenderData, Renderable};
use wgpu::Color;

use winit::platform::pump_events::EventLoopExtPumpEvents;

use super::{PyEventLoop, PySlider};

#[pyclass(unsendable)]
pub struct PyApplication {
    pub application: Option<Application>,
    pub event_loop_proxy: EventLoopProxy<CustomEvent>,
    pub renderables: Vec<Box<dyn Renderable>>,
    pub controls: Vec<Py<PySlider>>,
    pub update: Option<Py<PyFunction>>,
}

#[pymethods]
impl PyApplication {
    #[new]
    fn new(event_loop: &mut PyEventLoop) -> Self {
        let mut application = Self {
            application: None,
            renderables: Vec::new(),
            controls: Vec::new(),
            update: None,
            event_loop_proxy: event_loop
                .event_loop
                .as_ref()
                .expect("Event loop already consumed")
                .create_proxy(),
        };
        if let Some(inner_loop) = event_loop.event_loop.as_mut() {
            inner_loop.pump_app_events(Some(Duration::from_secs(0)), &mut application);
        }
        application
    }
}
impl ApplicationHandler<CustomEvent> for PyApplication {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = create_window(event_loop);
        create_application(window, &self.event_loop_proxy);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(ref mut application) = self.application else {
            return;
        };
        let Some(ref mut update) = self.update else {
            return;
        };
        application.window_event(window_id, &event);
        match event {
            WindowEvent::RedrawRequested => {
                {
                    let frame = application.next_frame();
                    let mut encoder = application.encoder();
                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    {
                        application.clear(&view, &mut encoder, Color::BLACK);
                        let mut render_data = RenderData {
                            view: &view,
                            multisampled_framebuffer: &application.multisampled_framebuffer,
                            depth_texture: &application.depth_texture,
                            encoder: &mut encoder,
                            camera: &application.camera,
                        };
                        for spheres in &self.renderables {
                            spheres.render(&mut render_data);
                        }
                    }
                    if !self.controls.is_empty() {
                        let raw_input = application
                            .egui_renderer
                            .state
                            .take_egui_input(&application.window);
                        let full_output =
                            application
                                .egui_renderer
                                .state
                                .egui_ctx()
                                .run(raw_input, |ctx| {
                                    egui::Window::new("Settings").show(ctx, |ui| {
                                        Python::attach(|py| {
                                            for slider in &mut self.controls {
                                                let mut slider_mut = slider.borrow_mut(py);
                                                let minimum = slider_mut.minimum;
                                                let maximum = slider_mut.maximum;
                                                ui.label(&slider_mut.name);
                                                ui.add(Slider::new(
                                                    &mut slider_mut.value,
                                                    minimum..=maximum,
                                                ));
                                            }
                                        });
                                    });
                                });

                        application.egui_renderer.state.handle_platform_output(
                            &application.window,
                            full_output.platform_output,
                        );

                        let tris = application.egui_renderer.state.egui_ctx().tessellate(
                            full_output.shapes,
                            application.window.scale_factor() as f32,
                        );
                        for (id, image_delta) in &full_output.textures_delta.set {
                            application.egui_renderer.renderer.update_texture(
                                &application.device,
                                &application.queue,
                                *id,
                                image_delta,
                            );
                        }
                        let screen_descriptor = ScreenDescriptor {
                            size_in_pixels: [application.config.width, application.config.height],
                            pixels_per_point: application.window.scale_factor() as f32,
                        };
                        application.egui_renderer.renderer.update_buffers(
                            &application.device,
                            &application.queue,
                            &mut encoder,
                            &tris,
                            &screen_descriptor,
                        );
                        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("egui"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                            })],
                            depth_stencil_attachment: None,
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        });
                        application.egui_renderer.renderer.render(
                            &mut render_pass.forget_lifetime(),
                            &tris,
                            &screen_descriptor,
                        );
                        for x in &full_output.textures_delta.free {
                            application.egui_renderer.renderer.free_texture(x)
                        }
                    }

                    application.queue.submit(Some(encoder.finish()));
                    frame.present();
                    application.update();
                    application.window.request_redraw();
                }
                Python::attach(|py| {
                    let result = update.call(py, (), None);
                    if let Err(err) = result {
                        println!("Could not call update: {err:?}");
                        println!("{}", err.traceback(py).unwrap().format().unwrap());
                    }
                });
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: CustomEvent) {
        match event {
            CustomEvent::Application(application) => self.application = Some(*application),
            CustomEvent::DropEvent(_) => {}
        }
    }
}
