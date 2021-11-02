use std::path::Path;

use structopt::StructOpt;
use visula::InstanceBinding;
use visula::SphereDelegate;
use visula_derive::delegate;
use wgpu::BufferUsages;
use winit::event::{KeyboardInput, VirtualKeyCode, WindowEvent};

use visula::{
    BindingBuilder, Buffer, DropEvent, InstanceHandle, MeshPipeline, Pipeline, Sphere, Spheres,
};

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
    spheres: Spheres,
    sphere_buffer: Buffer<Sphere>,
    mesh: MeshPipeline,
}

impl Simulation {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn handle_zdf(&mut self, application: &mut visula::Application, path: &Path) {
        let visula::io::zdf::ZdfFile {
            camera_center,
            point_cloud,
            mesh_vertex_buf,
            mesh_vertex_count,
        } = visula::io::zdf::read_zdf(path, &mut application.device);

        application.camera_controller.center = camera_center;
        self.sphere_buffer.update(application, &point_cloud[..]);
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
        let points = Spheres::new(
            application,
            &SphereDelegate {
                position: delegate!(sphere.position),
                radius: delegate!(0.1),
            },
        )
        .unwrap();
        let mesh = visula::create_mesh_pipeline(application).unwrap();
        let mut simulation = Simulation {
            render_mode: RenderMode::Points,
            sphere_buffer,
            spheres: points,
            mesh,
        };
        if let Some(filename) = &args.load_zdf {
            #[cfg(not(target_arch = "wasm32"))]
            simulation.handle_zdf(application, filename);
        }
        Ok(simulation)
    }

    fn update(&mut self, _: &visula::Application) {}

    fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>) {
        let bindings: &[&dyn InstanceBinding] = &[&self.sphere_buffer];
        match self.render_mode {
            RenderMode::Mesh => self.mesh.render(render_pass),
            RenderMode::Points => self.spheres.render(render_pass, bindings),
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
