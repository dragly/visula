use std::fs::File;

use glam::{Quat, Vec3};
use std::io::BufReader;

use clap::Parser;
use visula::{
    io::gltf::{parse_gltf, GltfMesh},
    MeshDelegate, MeshPipeline, RenderData,
};

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    filename: String,
}

struct Simulation {
    mesh_pipelines: Vec<MeshPipeline>,
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
        let cli = Cli::parse();
        let filename = cli.filename;
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        let gltf_file = parse_gltf(&mut reader, application)?;

        let mesh_pipelines: Vec<MeshPipeline> = gltf_file
            .scenes
            .into_iter()
            .flat_map(|scene| scene.meshes.into_iter())
            .map(|mesh| {
                let mut mesh_pipeline = MeshPipeline::new(
                    &application.rendering_descriptor(),
                    &MeshDelegate {
                        position: Vec3::new(0.0, 0.0, 0.0).into(),
                        rotation: Quat::IDENTITY.into(),
                        scale: Vec3::ONE.into(),
                    },
                )
                .unwrap();
                let GltfMesh {
                    vertex_buffer,
                    index_buffer,
                    index_count,
                } = mesh;

                mesh_pipeline.vertex_count = index_count;
                mesh_pipeline.vertex_buffer = vertex_buffer;
                mesh_pipeline.index_buffer = index_buffer;
                mesh_pipeline
            })
            .collect();
        let pipeline_count = mesh_pipelines.len();
        println!("Collected {pipeline_count} meshes");
        Ok(Simulation { mesh_pipelines })
    }
}

impl visula::Simulation for Simulation {
    type Error = Error;

    fn update(&mut self, _application: &mut visula::Application) {}

    fn render(&mut self, data: &mut RenderData) {
        for pipeline in &self.mesh_pipelines {
            pipeline.render(data);
        }
    }
}

fn main() {
    visula::run(|app| Simulation::new(app).expect("Initializing simulation failed"));
}
