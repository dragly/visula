use std::fs::File;

use glam::{Quat, Vec3};
use std::io::BufReader;
use structopt::StructOpt;

use visula::{
    io::gltf::{parse_gltf, GltfMesh},
    MeshDelegate, MeshPipeline, RenderData,
};

#[derive(StructOpt)]
struct Cli {
    #[structopt(long)]
    filename: String,
}

struct Simulation {
    mesh: MeshPipeline,
}

#[derive(Debug)]
struct Error {}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error {}
    }
}

impl From<visula::error::Error> for Error {
    fn from(_: visula::error::Error) -> Self {
        Error {}
    }
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Result<Simulation, Error> {
        let cli = Cli::from_args();
        let filename = cli.filename;
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        let gltf_file = parse_gltf(&mut reader, application)?;

        let mut mesh = MeshPipeline::new(
            &application.rendering_descriptor(),
            &MeshDelegate {
                position: Vec3::new(0.0, 0.0, 0.0).into(),
                rotation: Quat::IDENTITY.into(),
            },
        )
        .unwrap();
        let GltfMesh {
            vertex_buffer,
            index_buffer,
            index_count,
        } = gltf_file
            .scenes
            .into_iter()
            .next()
            .unwrap()
            .meshes
            .into_iter()
            .next()
            .unwrap();
        mesh.vertex_count = index_count;
        mesh.vertex_buffer = vertex_buffer;
        mesh.index_buffer = index_buffer;
        Ok(Simulation { mesh })
    }
}

impl visula::Simulation for Simulation {
    type Error = Error;

    fn update(&mut self, _application: &visula::Application) {}

    fn render(&mut self, data: &mut RenderData) {
        self.mesh.render(data);
    }
}

fn main() {
    visula::run(|app| Simulation::new(app).expect("Initializing simulation failed"));
}
