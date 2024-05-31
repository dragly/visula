from dataclasses import dataclass
import numpy as np
from itertools import combinations
from visula import LineDelegate, SphereDelegate, Figure, Slider, Uniform
import visula as vl
import numpy as np
import matplotlib.pyplot as plt

fig = Figure()


@dataclass
class Parameters(Uniform):
    cx: float
    cy: float
    fx: float
    fy: float


width = 32
height = 24
py, px = np.mgrid[0:height, 0:width]
py = vl.Expression(py.astype(np.float32))
px = vl.Expression(px.astype(np.float32))
parameters = Parameters(
    cx=width / 2,
    cy=height / 2,
    fx=100,
    fy=100,
)

uniform = parameters.instance()

u = (px - uniform.cx) / uniform.fx
v = (py - uniform.cy) / uniform.fy

start = vl.vec3(0.0, 0.0, 0.0)
end = vl.vec3(u, v, 1)

lines = LineDelegate(
    start=width * start,
    end=width * end,
    width=0.01,
    alpha=0.1,
)

spheres = SphereDelegate(
    position=width * end,
    radius=0.1,
    color=vl.vec3(0.3, 0.2, 0.8),
)

slider_cx = Slider(
    name="cx",
    value=width / 2,
    minimum=0,
    maximum=width,
    step=0.01,
)


def update():
    parameters.cx = slider_cx.value
    parameters.update()


fig.show(
    [spheres],
    update=update,
    controls=[slider_cx],
)
