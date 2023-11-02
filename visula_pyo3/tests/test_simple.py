# from visula._visula_pyo3 import testme, testyou

# def callback():
    # print("Hello")
    # testyou()

# testme(callback)

from visula import LineDelegate, SphereDelegate, Figure, Expression, InstanceBuffer
import numpy as np

a = 1.0
b = 100.0
c = 50.0

t = np.linspace(0, 2*3.14, 5000)
x = np.cos(a*t) + np.cos(b*t) / 2.0 + np.sin(c * t) / 3.0
y = np.sin(a*t) + np.sin(b*t) / 2.0 + np.cos(c * t) / 3.0
z = 2.0 * np.cos(t)
positions = 10.0 * np.array([x, y, z]).T

position = InstanceBuffer(positions)

spheres = SphereDelegate(
    position=position,
    radius=0.1,
    color=1.0 * position / 4.0 + 8.0 / 3.0,
)

fig = Figure()

def update():
    print("Update!")
    global a
    a += 1
    t = np.linspace(0, 2*3.14, 5000)
    x = np.cos(a*t) + np.cos(b*t) / 2.0 + np.sin(c * t) / 3.0
    y = np.sin(a*t) + np.sin(b*t) / 2.0 + np.cos(c * t) / 3.0
    z = 2.0 * np.cos(t)
    positions = 10.0 * np.array([x, y, z]).T
    position.update(positions)

fig.show([spheres], update=update)
