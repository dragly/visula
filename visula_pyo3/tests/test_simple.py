from dataclasses import dataclass
from visula_pyo3 import spawn
import numpy as np

@dataclass
class Spheres:


x = np.linspace(0, 10, 100)
y = np.sin(x)
z = np.cos(x)
positions = np.array([x, y, z])
print(positions)

# points = Points(positions)

spheres = Spheres(
    position=points,
)

# lines = Lines(
    # start=positions[:, :-1],
    # end=positions[:, 1:],
# )

spawn(positions[:, 1:])
