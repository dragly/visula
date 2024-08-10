import numpy as np
from itertools import combinations
from visula import LineDelegate, SphereDelegate, Figure
import visula as vl
import numpy as np


fig = Figure()

phi: float = (1 + np.sqrt(5)) / 2  # Golden ratio

vertices = np.array(
    [
        [-1.0, phi, 0.0],
        [1.0, phi, 0.0],
        [-1.0, -phi, 0.0],
        [1.0, -phi, 0.0],
        [0.0, -1.0, phi],
        [0.0, 1.0, phi],
        [0.0, -1.0, -phi],
        [0.0, 1.0, -phi],
        [phi, 0.0, -1.0],
        [phi, 0.0, 1.0],
        [-phi, 0.0, -1.0],
        [-phi, 0.0, 1.0],
    ]
)

faces = [
    [0, 11, 5],
    [0, 5, 1],
    [0, 1, 7],
    [0, 7, 10],
    [0, 10, 11],
    [1, 5, 9],
    [5, 11, 4],
    [11, 10, 2],
    [10, 7, 6],
    [7, 1, 8],
    [3, 9, 4],
    [3, 4, 2],
    [3, 2, 6],
    [3, 6, 8],
    [3, 8, 9],
    [4, 9, 5],
    [2, 4, 11],
    [6, 2, 10],
    [8, 6, 7],
    [9, 8, 1],
]

edges = set()
for face in faces:
    face_edges = list(combinations(face, 2))
    for edge in face_edges:
        edges.add(tuple(sorted(edge)))

edges = np.array(list(edge for edge in edges))

spheres = SphereDelegate(
    position=vertices,
    radius=0.2,
    color=vl.vec3(1.0, 1.0, 0.0),
)

lines = LineDelegate(
    start=vertices[edges[:, 0]],
    end=vertices[edges[:, 1]],
    width=0.1,
    color=vl.vec3(1.0, 1.0, 1.0),
)


def update():
    pass


fig.show([spheres, lines], update=update)
