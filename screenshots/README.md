Screenshots referenced from the project README.

Regenerate by running an example with `VISULA_SCREENSHOT` set to the desired output path. Optionally set `VISULA_SCREENSHOT_FRAMES` to control how many frames render before capture (default 30):

```bash
VISULA_SCREENSHOT=screenshots/showcase.png \
    cargo run --example showcase

VISULA_SCREENSHOT=screenshots/molecular_dynamics.png \
VISULA_SCREENSHOT_FRAMES=300 \
    cargo run --release --example molecular_dynamics -- --count 6

VISULA_SCREENSHOT=screenshots/python_spheres.png \
    ./run-python.sh visula_pyo3/examples/simple.py
```

The process exits automatically after the screenshot is written.
