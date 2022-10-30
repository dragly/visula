use std::fs::File;

use std::io::BufReader;
use structopt::StructOpt;

use visula::{
    io::gltf::{parse_gltf, GltfMesh, Scale},
    Buffer, Expression, ExpressionInner, Mesh, MeshDelegate, Pipeline, SimulationRenderData,
    Uniform,
};
use wgpu::BufferUsages;

#[derive(StructOpt)]
struct Cli {
    #[structopt(long)]
    filename: String,
}

struct Simulation {
    mesh: Mesh,
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

macro_rules! delegate_vec {
    ($($elements:expr),+) => {
        Expression::new(ExpressionInner::Vector {components: vec![ $($elements.into()),+ ]})
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

        let GltfMesh {
            vertex_buffer,
            vertex_count,
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

        let scale_input = &gltf_file
            .animations
            .iter()
            .next()
            .unwrap()
            .channels
            .iter()
            .next()
            .unwrap()
            .input_buffer;
        let scale_output = &gltf_file
            .animations
            .iter()
            .next()
            .unwrap()
            .channels
            .iter()
            .next()
            .unwrap()
            .output_buffer;

        let scale_data = Scale {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        };

        let scale_buffer = Buffer::<Scale>::new_with_init(
            application,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[scale_data],
            "scale",
        );

        let scale = scale_buffer.uniform();

        let mut mesh = Mesh::new(
            application,
            &MeshDelegate {
                position: delegate_vec!(0.0, 0.0, 0.0),
                scale: delegate_vec!(scale.x, 1.0, 1.0),
            },
        )
        .unwrap();
        mesh.vertex_count = vertex_count;
        mesh.index_count = index_count;
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
