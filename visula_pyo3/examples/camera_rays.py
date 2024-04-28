import numpy as np
from itertools import combinations
from visula import LineDelegate, SphereDelegate, Figure
import visula as vl
import numpy as np
import matplotlib.pyplot as plt

fig = Figure()

width = 32
height = 24
py, px = np.mgrid[0:height, 0:width]
cx = width / 2
cy = height / 2
fx = 100
fy = 100
u = (px - cx) / fx
v = (py - cy) / fy

start = np.array([np.zeros_like(u), np.zeros_like(u), np.zeros_like(u)]).transpose().reshape(-1, 3)
end = np.array([u, v, np.ones_like(u)]).transpose().reshape(-1, 3)

print(start[0])
print(end[0])
print(start[100])
print(end[100])

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

def update():
    pass


fig.show([spheres], update=update,)

