use std::path::Path;

use bytemuck::{Pod, Zeroable};
use structopt::StructOpt;
use wgpu::BufferUsages;
use winit::event::{KeyboardInput, VirtualKeyCode, WindowEvent};

use visula::{
    BindingBuilder, Buffer, BufferInner, DropEvent, MeshPipeline, Pipeline, SimulationRenderData,
    Sphere, SphereDelegate, Spheres, Uniform, UniformBinding, UniformField, UniformHandle,
};
use visula_derive::{delegate, Uniform};

#[derive(StructOpt)]
struct Cli {
    #[structopt(long)]
    load_zdf: Option<std::path::PathBuf>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum RenderMode {
    Points,
    Mesh,
}

impl std::fmt::Display for RenderMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
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
    sphere_buffer: Buffer<Sphere>,
    settings: Settings,
    settings_buffer: Buffer<Settings>,
    mesh: MeshPipeline,
}

impl Simulation {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn handle_zdf(&mut self, application: &mut visula::Application, path: &Path) {
        let visula::io::zdf::ZdfFile {
            camera_center,
            point_cloud,
            mesh_vertex_buf,
            mesh_index_buf,
            mesh_vertex_count,
        } = visula::io::zdf::read_zdf(path, &mut application.device);

        application.camera_controller.center = camera_center;
        self.sphere_buffer.update(application, &point_cloud[..]);
        self.mesh.index_buf = mesh_index_buf;
        self.mesh.vertex_buf = mesh_vertex_buf;
        self.mesh.vertex_count = mesh_vertex_count;
    }

    pub fn handle_xyz(
        &mut self,
        application: &mut visula::Application,
        DropEvent { text, .. }: &DropEvent,
    ) {
        let visula::io::xyz::XyzFile { point_cloud } =
            visula::io::xyz::read_xyz(text, &mut application.device);

        self.sphere_buffer.update(application, &point_cloud[..]);
    }
}

#[derive(Debug)]
struct Error {}

impl visula::Simulation for Simulation {
    type Error = Error;
    fn init(application: &mut visula::Application) -> Result<Simulation, Error> {
        let args = Cli::from_args();
        let sphere_buffer = Buffer::<Sphere>::new(
            application,
            BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::COPY_DST,
            "point",
        );
        let sphere = sphere_buffer.instance();
        let settings_data = Settings {
            radius: 0.5,
            _padding: 0.0,
            _padding2: 0.0,
            _padding3: 0.0,
        };
        let settings_buffer = Buffer::new_with_init(
            application,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[settings_data],
            "settings",
        );
        let settings = settings_buffer.uniform();
        let points = Spheres::new(
            application,
            &SphereDelegate {
                position: delegate!(sphere.position),
                radius: delegate!(settings.radius),
                color: delegate!(sphere.color),
            },
        )
        .unwrap();
        let mesh = visula::create_mesh_pipeline(application).unwrap();
        let mut simulation = Simulation {
            render_mode: RenderMode::Points,
            sphere_buffer,
            spheres: points,
            mesh,
            settings: settings_data,
            settings_buffer,
        };
        if let Some(filename) = &args.load_zdf {
            #[cfg(not(target_arch = "wasm32"))]
            simulation.handle_zdf(application, filename);
        }
        Ok(simulation)
    }

    fn update(&mut self, application: &visula::Application) {
        self.settings_buffer.update(application, &[self.settings]);
    }

    fn render(&mut self, data: &mut SimulationRenderData) {
        match self.render_mode {
            RenderMode::Mesh => self.mesh.render(data),
            RenderMode::Points => self.spheres.render(data),
        };
    }

    fn handle_event(&mut self, app: &mut visula::Application, event: &WindowEvent) {
        match event {
            WindowEvent::DroppedFile(path) => {
                log::info!("Dropped file {:?}", path);
                let bytes = std::fs::read(&path).unwrap();
                let drop_event = DropEvent {
                    name: path.to_str().unwrap().to_string(),
                    text: bytes,
                };
                if let Some(extension) = path.extension() {
                    if let Some(extension) = extension.to_str() {
                        match extension {
                            "xyz" => self.handle_xyz(app, &drop_event),
                            #[cfg(not(target_arch = "wasm32"))]
                            "zdf" => self.handle_zdf(app, path),
                            _ => {
                                log::warn!("Unsupported format {}", extension);
                            }
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::M),
                        state: winit::event::ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.render_mode = match self.render_mode {
                    RenderMode::Mesh => RenderMode::Points,
                    RenderMode::Points => RenderMode::Mesh,
                }
            }
            _ => {}
        }
    }

    fn gui(&mut self, context: &egui::Context) {
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
    visula::run::<Simulation>();
}
