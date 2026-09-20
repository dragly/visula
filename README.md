# Visula

Visula is a visualization library based on [wgpu](https://wgpu.rs).
It was made to make it easy to express data visualizations.
I am using it for basic visualizations,
interactive presentations, demos, applications and games.

The library is designed around my own needs, but has been shared in case
others find it useful too.

Visula works natively in Linux, Windows or macOS,
or can be used to target the web with WASM and WebGPU.

![Showcase](screenshots/showcase.png)

## Python example

The idea is to make it easy to create data-driven visualizations.
Primitives like spheres, lines or triangle meshes can be defined directly from data.
InstanceBuffers can be used to define multiple instances of a given primitive:


```python
from visula import Spheres, Figure, InstanceBuffer
import visula as vl
import numpy as np

t = InstanceBuffer(np.linspace(0, 100, 100_000))
position = 10.0 * vl.vec3(vl.cos(t), vl.sin(t), t / 50.0 - 1.0)

spheres = Spheres(
    position=position,
    radius=0.2,
    color=vl.colormap(t / 100.0, "viridis"),
)

Figure().show([spheres])
```

Here, `position`, `radius` and `color` are all expressions.
Visula compiles these into the shader and evaluates them per instance on the GPU.
This means that there is only one array `t` uploaded to the GPU.

![Python spheres](screenshots/python_spheres.png)

## Rust example

The same visualization in Rust:

```rust
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
```



## Run the examples

```bash
# Native Rust
cargo run --example spheres
cargo run --example showcase
cargo run --example molecular_dynamics
cargo run --example neuron

# Python (uv sync builds the Rust extension on first run)
uv run visula_pyo3/examples/simple.py
uv run visula_pyo3/examples/controls.py

# Web (WebGPU/WebGL)
./run-wasm.sh
```

Below is a screenshot of the molecular dynamics example:

![Molecular dynamics](screenshots/molecular_dynamics.png)

See `visula/examples/` and `visula_pyo3/examples/` for more examples.

## License

Apache-2.0. See [LICENSE](LICENSE).
