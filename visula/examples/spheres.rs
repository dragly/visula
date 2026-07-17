use visula::{colormap, vec3, Colormap, SphereGeometry, SphereMaterial, Spheres};

fn main() {
    visula::run(|application| {
        let data: Vec<f32> = (0..100_000).map(|i| i as f32 * 0.001).collect();
        let t = application.instances(&data);
        let position = 10.0 * vec3(t.cos(), t.sin(), &t / 50.0 - 1.0);
        Spheres::new(
            &application.rendering_descriptor(),
            &SphereGeometry {
                position,
                radius: 0.2.into(),
                color: colormap(&t / 100.0, Colormap::Viridis),
            },
            &SphereMaterial::default(),
        )
        .unwrap()
    });
}
