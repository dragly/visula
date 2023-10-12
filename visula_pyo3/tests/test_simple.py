from dataclasses import dataclass, field
from typing import Any
from visula_pyo3 import spawn, Expression, Points, Spheres
import numpy as np

x = np.linspace(0, 10, 100)
y = np.sin(x)
z = np.cos(x)
positions = 10.0 * np.array([x, y, z]).T

points = Points(positions)

spheres = Spheres(
    position=points.position,
    radius=1.0,
    color=[1.0, 0.0, 1.0],
)

spawn(spheres, points)
