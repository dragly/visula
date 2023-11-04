# from visula._visula_pyo3 import testme, testyou

# def callback():
# print("Hello")
# testyou()

# testme(callback)

from dataclasses import dataclass
from visula import LineDelegate, SphereDelegate, Figure, Expression, InstanceBuffer, Uniform
import visula as vl
import numpy as np

count = 1000000


@dataclass
class Parameters(Uniform):
    a: np.float32
    b: np.float32
    c: np.float32

    def __post_init__(self):
        super().__init__()


parameters = Parameters(
    a=10.0,
    b=100.0,
    c=50.0,
)

parameters_uniform = parameters.instance()
parameters.update()

def create_particles(t, a, b, c):
    a = 10.0 * vl.cos(a)
    b = 100.0 * vl.sin(b)
    c = 50.0 * vl.cos(c)
    d = 8000
    x = vl.cos(a * t) + vl.cos(b * t) / 2.0 + vl.sin(c * t) / 3.0 + vl.cos(d * t) / 20.0
    y = vl.sin(a * t) + vl.sin(b * t) / 2.0 + vl.cos(c * t) / 3.0 + vl.sin(d * t) / 20.0
    z = 2.0 * vl.cos(t) + vl.cos(2 * d * t) / 20.0 + vl.sin(2 * d * t) / 20.0
    positions = 10.0 * vl.vec3(x, y, z)
    return positions


t = InstanceBuffer(np.linspace(0, 2 * 3.14, count))
position = create_particles(t, parameters_uniform.a, parameters_uniform.b, parameters_uniform.c)

spheres = SphereDelegate(
    position=position,
    radius=0.01,
    color=1.0 * position / 4.0 + 8.0 / 3.0,
)

fig = Figure()


def update():
    global a
    global b
    global c
    parameters.a += 0.0001
    parameters.b += 0.00001
    parameters.c += 0.00001
    parameters.update()


fig.show([spheres], update=update)
