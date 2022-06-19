use std::fs::File;

use std::io::BufReader;
use structopt::StructOpt;

use visula::{
    io::gltf::{parse_gltf, GltfMesh},
    MeshPipeline, Pipeline, SimulationRenderData,
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

impl visula::Simulation for Simulation {
    type Error = Error;
    fn init(application: &mut visula::Application) -> Result<Simulation, Error> {
        let cli = Cli::from_args();
        let filename = cli.filename;
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        let gltf_file = parse_gltf(&mut reader, application)?;

        let mut mesh = visula::create_mesh_pipeline(application).unwrap();
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
        mesh.vertex_buf = vertex_buffer;
        mesh.index_buf = index_buffer;
        Ok(Simulation { mesh })
    }

    fn update(&mut self, _application: &visula::Application) {}

    fn render(&mut self, data: &mut SimulationRenderData) {
        self.mesh.render(data);
    }
}

fn main() {
    visula::run::<Simulation>();
}
