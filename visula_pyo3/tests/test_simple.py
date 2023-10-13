from dataclasses import dataclass, field
from typing import Any
from visula import Expression, Points, Spheres, Figure
import numpy as np

t = np.linspace(0, 2*3.14, 10000)
x = np.cos(t)**3 + t
y = np.sin(t)**3
z = np.cos(t)**3
positions = 10.0 * np.array([x, y, z]).T

fig = Figure()
points = Points(positions)

spheres = Spheres(
    position=points.position,
    radius=1.0,
    color=[1.0, 0.0, 1.0],
)
fig.show(spheres, points)

print("Hello")

t = np.linspace(0, 2*3.14, 10000)
x = np.sin(t)**3
y = np.sin(t)**2
z = np.cos(t)**1
positions = 10.0 * np.array([x, y, z]).T

fig = Figure()
points = Points(positions)

spheres = Spheres(
    position=points.position,
    radius=1.0,
    color=[0.0, 1.0, 1.0],
)
fig.show(spheres, points)
