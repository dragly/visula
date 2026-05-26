# Visula

Turn data streams from simulations and recordings into interactive 3D visualizations you can share on the web.

Visula is a scientific visualization library built on [wgpu](https://wgpu.rs).
Applications can run natively in Linux, Windows or macOS, or target the web with WASM and WebGPU.

Visula is built around my own visualization needs and is shared in case it's useful to others. It's a work in progress — APIs may change.

![Showcase](screenshots/showcase.png)

## Python example

The idea behind Visula is to make it easy to create data-driven visualizations.
Primitives like spheres, lines or triangle meshes can be defined directly from data.
This includes their position and color.

InstanceBuffers can be used to define multiple instances of a given primitive:


```python
from visula import SphereDelegate, Figure, InstanceBuffer
import visula as vl
import numpy as np

t = InstanceBuffer(np.linspace(0, 100, 100_000))
position = 10.0 * vl.vec3(vl.cos(t), vl.sin(t), t)

spheres = SphereDelegate(
    position=position,
    radius=0.2,
    color=position / 4.0,
)

Figure().show([spheres])
```

Here, `position`, `radius` and `color` are all expressions.
Visula compiles these into the shader and evaluates them per instance on the GPU.
This means that there is only one array `t` uploaded to the GPU.

![Python spheres](screenshots/python_spheres.png)

## Rust example

```rust
use visula::{Expression, InstanceDeviceExt, SphereGeometry, SphereMaterial, Spheres};
use visula_derive::Instance;

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    position: glam::Vec3,
    _padding: f32,
}

let buffer = application.device.create_instance_buffer::<Particle>();
let particle = buffer.instance();

let spheres = Spheres::new(
    &application.rendering_descriptor(),
    &SphereGeometry {
        position: particle.position,
        radius: 0.5.into(),
        color: Expression::Position * 0.1 + 0.5,
    },
    &SphereMaterial { color: Expression::InputColor.lit() },
)?;
```

![Molecular dynamics](screenshots/molecular_dynamics.png)

## Run the examples

```bash
# Native Rust
cargo run --example showcase
cargo run --example molecular_dynamics
cargo run --example neuron

# Python (uv sync builds the Rust extension on first run)
uv run visula_pyo3/examples/simple.py
uv run visula_pyo3/examples/controls.py

# Web (WebGPU/WebGL)
./run-wasm.sh
```

See `visula/examples/` and `visula_pyo3/examples/` for the full set.

## License

Apache-2.0. See [LICENSE](LICENSE).
