use glam::Vec3;
use visula::{
    painter::{Painter, Sphere},
    RenderData,
};

struct Simulation {
    painter: Painter,
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Result<Simulation, Error> {
        Ok(Simulation {
            painter: Painter::new(application),
        })
    }
}

#[derive(Debug)]
struct Error;

impl visula::Simulation for Simulation {
    type Error = Error;
    fn update(&mut self, application: &mut visula::Application) {
        self.painter.spheres(&[Sphere {
            position: Vec3::ZERO,
            color: Vec3::new(0.2, 0.7, 0.9),
            radius: 4.0,
        }]);

        self.painter.update(application);
    }

    fn render(&mut self, render_data: &mut RenderData) {
        self.painter.render(render_data);
    }
}

fn main() {
    visula::run(|app| Simulation::new(app).expect("Initializing simulation failed"));
}
