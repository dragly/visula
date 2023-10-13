from dataclasses import dataclass, field
from typing import Any
from visula_pyo3 import spawn, Expression, Points, Spheres
import numpy as np

t = np.linspace(0, 2*3.14, 10000)
x = np.cos(t)**3 + t
y = np.sin(t)**3
z = np.cos(t)**3
positions = 10.0 * np.array([x, y, z]).T

points = Points(positions)

spheres = Spheres(
    position=points.position,
    radius=1.0,
    color=[1.0, 0.0, 1.0],
)

spawn(spheres, points)
