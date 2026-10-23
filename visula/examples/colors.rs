use visula::post_process::config::{SkyMode, Tonemapping};
use visula::{
    glam::Vec3, srgb, wgpu, Expression, RenderData, SphereGeometry, SphereMaterial, Spheres,
};

struct Simulation {
    spheres: Vec<Spheres>,
}

impl Simulation {
    fn new(application: &mut visula::Application) -> Simulation {
        let sphere = |x: f32, color: Vec3| {
            Spheres::new(
                &application.rendering_descriptor(),
                &SphereGeometry {
                    position: Vec3::new(x, 0.0, 0.0).into(),
                    radius: 15.0.into(),
                    color: color.into(),
                },
                &SphereMaterial {
                    color: Expression::InputColor,
                },
            )
            .unwrap()
        };
        Simulation {
            spheres: vec![sphere(-20.0, srgb(0x6E8FC9)), sphere(20.0, srgb(0xD8865A))],
        }
    }
}

impl visula::Simulation for Simulation {
    fn update(&mut self, application: &mut visula::Application) {
        application.post_processor.config.tonemapping = Tonemapping::None;
        application.post_processor.config.sky.mode = SkyMode::Off;
    }

    fn render(&mut self, data: &mut RenderData) {
        for spheres in &mut self.spheres {
            spheres.render(data);
        }
    }

    fn clear_color(&self) -> wgpu::Color {
        let page = srgb(0xF6F5F0);
        wgpu::Color {
            r: page.x as f64,
            g: page.y as f64,
            b: page.z as f64,
            a: 1.0,
        }
    }
}

fn main() {
    visula::run(Simulation::new);
}
