from visula import LineDelegate, SphereDelegate, Figure
import numpy as np

t = np.linspace(0, 2*3.14, 10000)
x = np.cos(t)**3 + t
y = np.sin(t)**3
z = np.cos(t)**3
positions = 10.0 * np.array([x, y, z]).T

spheres = SphereDelegate(
    position=positions,
    radius=1.0,
    color=[1.0, 0.0, 1.0],
)

t = np.linspace(0, 2*3.14, 10000)
x = np.sin(t)**3
y = np.sin(t)**2
z = np.cos(t)**1
positions = 10.0 * np.array([x, y, z]).T

fig = Figure()

lines = LineDelegate(
    start=positions[:-1],
    end=positions[1:],
    width=1.0,
    alpha=1.0,
)

def hello(first, second):
    print(f"Got {first=} {second=}")

fig.show([spheres, lines], callback=hello)
