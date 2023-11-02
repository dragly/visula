# from visula._visula_pyo3 import testme, testyou

# def callback():
    # print("Hello")
    # testyou()

# testme(callback)

from visula import LineDelegate, SphereDelegate, Figure, Expression, InstanceBuffer
import numpy as np

a = 0
b = 0
c = 0
count = 10000

def create_particles(a, b, c):
    a = 10.0 * np.cos(a)
    b = 100.0 * np.sin(b)
    c = 50.0 * np.cos(b)
    d = 8000
    t = np.linspace(0, 2*3.14, count)
    x = np.cos(a*t) + np.cos(b*t) / 2.0 + np.sin(c * t) / 3.0 + np.cos(d * t) / 20.0
    y = np.sin(a*t) + np.sin(b*t) / 2.0 + np.cos(c * t) / 3.0 + np.sin(d * t) / 20.0
    z = 2.0 * np.cos(t) + np.cos(2 * d * t) / 20.0 + np.sin(2 *d * t) / 20.0
    positions = 10.0 * np.array([x, y, z]).T
    return positions

positions = create_particles(a, b, c)
position = InstanceBuffer(positions)

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
    positions = create_particles(a, b, c)
    position.update(positions)

fig.show([spheres], update=update)
