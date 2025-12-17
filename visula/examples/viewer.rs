#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
use std::{io::Cursor, path::Path};
use visula::{MeshMaterial, Renderable};

use bytemuck::{Pod, Zeroable};
use clap::Parser;
use glam::{Quat, Vec3};
use oxifive::ReadSeek;
use winit::event::{Event, KeyEvent, WindowEvent};

use glam::Vec4;
use visula::{
    CustomEvent, DropEvent, InstanceBuffer, MeshGeometry, MeshPipeline, RenderData, SphereDelegate,
    SpherePrimitive, Spheres, UniformBuffer,
};
use visula_derive::Uniform;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    load_zdf: Option<std::path::PathBuf>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum RenderMode {
    Points,
    Mesh,
}

impl std::fmt::Display for RenderMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Uniform, Zeroable)]
struct Settings {
    radius: f32,
    _padding: f32,
    _padding2: f32,
    _padding3: f32,
}

struct Simulation {
    render_mode: RenderMode,
    spheres: Spheres,
    sphere_buffer: InstanceBuffer<SpherePrimitive>,
    settings: Settings,
    settings_buffer: UniformBuffer<Settings>,
    mesh: MeshPipeline,
}

impl Simulation {
    pub fn handle_zdf<R: ReadSeek>(&mut self, application: &mut visula::Application, input: R) {
        let visula::io::zdf::ZdfFile {
            camera_center,
            point_cloud,
            mesh_vertex_buf,
            mesh_index_buf,
            mesh_vertex_count,
        } = visula::io::zdf::read_zdf(input, &mut application.device);

        application.camera_controller.current_transform.center = camera_center;
        application.camera_controller.target_transform.center = camera_center;

        self.sphere_buffer
            .update(&application.device, &application.queue, &point_cloud[..]);
        self.mesh.index_buffer = mesh_index_buf;
        self.mesh.vertex_buffer = mesh_vertex_buf;
        self.mesh.vertex_count = mesh_vertex_count;
    }
}

#[derive(Debug)]
struct Error {}

impl Simulation {
    fn new(application: &mut visula::Application) -> Result<Simulation, Error> {
        #[cfg(not(target_arch = "wasm32"))]
        let args = Cli::parse();
        let sphere_buffer = InstanceBuffer::<SpherePrimitive>::new(&application.device);
        let sphere = sphere_buffer.instance();
        let settings_data = Settings {
            radius: 0.5,
            _padding: 0.0,
            _padding2: 0.0,
            _padding3: 0.0,
        };
        let settings_buffer = UniformBuffer::new_with_init(&application.device, &settings_data);
        let settings = settings_buffer.uniform();
        let points = Spheres::new(
            &application.rendering_descriptor(),
            &SphereDelegate {
                position: sphere.position,
                radius: settings.radius,
                color: sphere.color,
            },
        )
        .unwrap();
        let mesh = MeshPipeline::new(
            &application.rendering_descriptor(),
            &MeshGeometry {
                position: Vec3::new(0.0, 0.0, 0.0).into(),
                rotation: Quat::IDENTITY.into(),
                scale: Vec3::ONE.into(),
            },
            &MeshMaterial {
                color: Vec4::splat(1.0).into(),
            },
        )
        .unwrap();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut simulation = Simulation {
                render_mode: RenderMode::Points,
                sphere_buffer,
                spheres: points,
                mesh,
                settings: settings_data,
                settings_buffer,
            };
            if let Some(filename) = &args.load_zdf {
                let input = File::open(filename).unwrap();
                simulation.handle_zdf(application, input);
            }
            Ok(simulation)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let simulation = Simulation {
                render_mode: RenderMode::Points,
                sphere_buffer,
                spheres: points,
                mesh,
                settings: settings_data,
                settings_buffer,
            };
            Ok(simulation)
        }
    }
}

impl visula::Simulation for Simulation {
    type Error = Error;

    fn update(&mut self, application: &mut visula::Application) {
        self.settings_buffer
            .update(&application.queue, &self.settings);
    }

    fn render(&mut self, data: &mut RenderData) {
        match self.render_mode {
            RenderMode::Mesh => self.mesh.render(data),
            RenderMode::Points => self.spheres.render(data),
        };
    }

    fn handle_event(&mut self, app: &mut visula::Application, event: &Event<CustomEvent>) {
        match event {
            Event::WindowEvent {
                event: ref window_event,
                ..
            } => match window_event {
                WindowEvent::DroppedFile(path) => {
                    log::info!("Dropped file {path:?}");
                    let bytes = std::fs::read(path).unwrap();
                    let drop_event = DropEvent {
                        name: path.to_str().unwrap().to_string(),
                        bytes,
                    };
                    if let Some(extension) = path.extension() {
                        if let Some(extension) = extension.to_str() {
                            match extension {
                                "zdf" => {
                                    let input = Cursor::new(drop_event.bytes.clone());
                                    self.handle_zdf(app, input);
                                }
                                _ => {
                                    log::warn!("Unsupported format {extension}");
                                }
                            }
                        }
                    }
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            logical_key,
                            state: winit::event::ElementState::Pressed,
                            ..
                        },
                    ..
                } if logical_key == "m" => {
                    self.render_mode = match self.render_mode {
                        RenderMode::Mesh => RenderMode::Points,
                        RenderMode::Points => RenderMode::Mesh,
                    }
                }
                _ => {}
            },
            Event::UserEvent(CustomEvent::DropEvent(drop_event)) => {
                log::info!("Dropped file custom {:?}", drop_event.name);
                let path = Path::new(&drop_event.name);
                if let Some(extension) = path.extension() {
                    if let Some(extension) = extension.to_str() {
                        match extension {
                            "zdf" => {
                                let input = Box::new(Cursor::new(drop_event.bytes.clone()));
                                self.handle_zdf(app, input);
                            }
                            _ => {
                                log::warn!("Unsupported format {extension}");
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn gui(&mut self, _application: &visula::Application, context: &egui::Context) {
        egui::Window::new("Settings").show(context, |ui| {
            ui.label("Radius");
            ui.add(egui::Slider::new(&mut self.settings.radius, 0.1..=2.5));
            ui.label("Render mode");
            for option in [RenderMode::Points, RenderMode::Mesh] {
                if ui
                    .selectable_label(self.render_mode == option, option.to_string())
                    .clicked()
                {
                    self.render_mode = option;
                }
            }
        });
    }
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    visula::run(|app| Simulation::new(app).expect("Initializing simulation failed"));
}
