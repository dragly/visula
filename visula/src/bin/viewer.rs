use std::{fs::File, io::Cursor, path::Path};

use oxifive::ReadSeek;
use structopt::StructOpt;
use winit::event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use visula::{CustomEvent, DropEvent, InstancedPipeline, MeshPipeline, Pipeline};

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
    //#[cfg(not(target_arch = "wasm32"))]
    pub fn handle_zdf(
        &mut self,
        application: &mut visula::Application,
        input: Box<dyn ReadSeek>,
        //DropEvent { input, .. }: &DropEvent,
        //path: &Path
    ) {
        let visula::io::zdf::ZdfFile {
            camera_center,
            instance_buffer,
            instance_count,
            mesh_vertex_buf,
            mesh_vertex_count,
        } = visula::io::zdf::read_zdf(input, &mut application.device);

        application.camera_controller.center = camera_center;
        self.points.instance_buffer = instance_buffer;
        self.points.instance_count = instance_count;
        self.mesh.vertex_buf = mesh_vertex_buf;
        self.mesh.vertex_count = mesh_vertex_count;
    }

    pub fn handle_xyz(&mut self, application: &mut visula::Application, text: &Vec<u8>) {
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
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(filename) = &args.load_zdf {
            let input = File::open(filename).unwrap();
            simulation.handle_zdf(application, Box::new(input));
        }
        simulation
    }

    fn update(&mut self, application: &visula::Application) {}

    fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>) {
        match self.render_mode {
            RenderMode::Mesh => self.mesh.render(render_pass),
            RenderMode::Points => self.points.render(render_pass),
        };
    }

    fn handle_event(&mut self, app: &mut visula::Application, event: &Event<CustomEvent>) {
        match event {
            Event::UserEvent(user_event) => {
                match user_event {
                    CustomEvent::DropEvent(drop_event) => {
                        log::info!("Dropped file custom {:?}", drop_event.name);
                        let path = Path::new(&drop_event.name);
                        if let Some(extension) = path.extension() {
                            if let Some(extension) = extension.to_str() {
                                match extension {
                                    "xyz" => self.handle_xyz(app, &drop_event.bytes),
                                    //#[cfg(not(target_arch = "wasm32"))]
                                    "zdf" => {
                                        let input = Box::new(Cursor::new(drop_event.bytes.clone()));
                                        self.handle_zdf(app, input);
                                    }
                                    _ => {
                                        log::warn!("Unsupported format {}", extension);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::WindowEvent {
                event: ref window_event,
                ..
            } => {
                match window_event {
                    WindowEvent::DroppedFile(path) => {
                        log::info!("Dropped file {:?}", path);
                        let bytes = std::fs::read(&path).unwrap();
                        if let Some(extension) = path.extension() {
                            if let Some(extension) = extension.to_str() {
                                match extension {
                                    "xyz" => self.handle_xyz(app, &bytes),
                                    //#[cfg(not(target_arch = "wasm32"))]
                                    "zdf" => {
                                        let input = Box::new(Cursor::new(bytes));
                                        self.handle_zdf(app, input);
                                    }
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
            _ => {}
        }
    }
}

fn main() {
    visula::run::<Simulation>();
}
