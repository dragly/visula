# from visula._visula_pyo3 import testme, testyou

# def callback():
# print("Hello")
# testyou()

# testme(callback)

from dataclasses import dataclass
from visula import LineDelegate, SphereDelegate, Figure, Expression, InstanceBuffer, Uniform
import visula as vl
import numpy as np

a = 0
b = 0
c = 0
count = 10000


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

print(parameters_uniform.a)


def create_particles(t, a, b, c):
    a = 10.0 * vl.cos(a)
    b = 100.0 * vl.sin(b)
    c = 50.0 * vl.cos(b)
    d = 8000
    x = vl.cos(a * t) + vl.cos(b * t) / 2.0 + vl.sin(c * t) / 3.0 + vl.cos(d * t) / 20.0
    y = vl.sin(a * t) + vl.sin(b * t) / 2.0 + vl.cos(c * t) / 3.0 + vl.sin(d * t) / 20.0
    z = 2.0 * vl.cos(t) + vl.cos(2 * d * t) / 20.0 + vl.sin(2 * d * t) / 20.0
    positions = 10.0 * vl.vec3(x, y, z)
    return positions


t = InstanceBuffer(np.linspace(0, 2 * 3.14, count))
position = create_particles(t, a, b, c)

spheres = SphereDelegate(
    position=position,
    radius=0.1,
    color=1.0 * position / 4.0 + 8.0 / 3.0,
)

fig = Figure()


def update():
    global a
    global b
    global c
    a += 0.00001
    b += 0.000001
    c += 0.00001
    # positions = create_particles(a, b, c)
    # position.update(positions)


fig.show([spheres], update=update)
