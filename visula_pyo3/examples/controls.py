from dataclasses import dataclass
from visula import SphereDelegate, Figure, InstanceBuffer, Uniform, Slider
import visula as vl
import numpy as np


count = 100000


@dataclass
class Parameters(Uniform):
    a: float
    b: float
    c: float


parameters = Parameters(
    a=0.0,
    b=0.0,
    c=0.0,
)

parameters_uniform = parameters.instance()
parameters.update()


def create_particles(t, a, b, c):
    a = 10.0 * vl.cos(a)
    b = 100.0 * vl.sin(b)
    c = 50.0 * vl.cos(b)
    x = vl.cos(a * t) + vl.cos(b * t) / 2.0 + vl.sin(c * t) / 3.0
    y = vl.sin(a * t) + vl.sin(b * t) / 2.0 + vl.cos(c * t) / 3.0
    z = t
    positions = 10.0 * vl.vec3(x, y, z)
    return positions


t = InstanceBuffer(np.linspace(0, 3.14 * count / 1000, count))
position = create_particles(t, parameters_uniform.a, parameters_uniform.b, parameters_uniform.c)

spheres = SphereDelegate(
    position=position,
    radius=0.2,
    color=1.0 * position / 4.0 + 8.0 / 3.0,
)

fig = Figure()


a_slider = Slider(
    name="a",
    value=0.0,
    minimum=0.0,
    maximum=10.0,
    step=0.1,
)

controls = [
    a_slider,
]


def update():
    parameters.a = a_slider.value
    parameters.update()


fig.show(
    [spheres],
    update=update,
    controls=controls,
)
