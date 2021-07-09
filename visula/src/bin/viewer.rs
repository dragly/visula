use std::path::Path;

use structopt::StructOpt;
use winit::event::{KeyboardInput, VirtualKeyCode, WindowEvent};

use visula::{DropEvent, InstancedPipeline, MeshPipeline, Pipeline};

#[derive(StructOpt)]
struct Cli {
    #[structopt(long)]
    load_zdf: Option<std::path::PathBuf>,
}

enum RenderMode {
    Points,
    Mesh,
}

struct Simulation {
    render_mode: RenderMode,
    points: InstancedPipeline,
    mesh: MeshPipeline,
}

impl Simulation {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn handle_zdf(&mut self, application: &mut visula::Application, path: &Path) {
        let visula::io::zdf::ZdfFile {
            camera_center,
            instance_buffer,
            instance_count,
            mesh_vertex_buf,
            mesh_vertex_count,
        } = visula::io::zdf::read_zdf(path, &mut application.device);

        application.camera_controller.center = camera_center;
        self.points.instance_buffer = instance_buffer;
        self.points.instance_count = instance_count;
        self.mesh.vertex_buf = mesh_vertex_buf;
        self.mesh.vertex_count = mesh_vertex_count;
    }

    pub fn handle_xyz(
        &mut self,
        application: &mut visula::Application,
        DropEvent { text, .. }: &DropEvent,
    ) {
        let visula::io::xyz::XyzFile {
            instance_buffer,
            instance_count,
        } = visula::io::xyz::read_xyz(text, &mut application.device);

        self.points.instance_buffer = instance_buffer;
        self.points.instance_count = instance_count;
    }
}

impl visula::Simulation for Simulation {
    fn init(application: &mut visula::Application) -> Simulation {
        let args = Cli::from_args();
        let points = visula::create_spheres_pipeline(application).unwrap();
        let mesh = visula::create_mesh_pipeline(application).unwrap();
        let mut simulation = Simulation {
            render_mode: RenderMode::Points,
            points,
            mesh,
        };
        if let Some(filename) = &args.load_zdf {
            #[cfg(not(target_arch = "wasm32"))]
            simulation.handle_zdf(application, filename);
        }
        simulation
    }

    fn update(&mut self, _: &visula::Application) {}

    fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>) {
        match self.render_mode {
            RenderMode::Mesh => self.mesh.render(render_pass),
            RenderMode::Points => self.points.render(render_pass),
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
}

fn main() {
    visula::run::<Simulation>();
}
