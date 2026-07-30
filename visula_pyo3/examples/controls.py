from visula import Spheres, Figure, InstanceBuffer, Slider
import visula as vl
import numpy as np

count = 100000

a = Slider("a", value=0.0, minimum=0.0, maximum=1.0, step=0.1)
b = Slider("b", value=0.0, minimum=0.0, maximum=1.0, step=0.1)
c = Slider("c", value=0.0, minimum=0.0, maximum=1.0, step=0.1)
radius = Slider("radius", value=0.2, minimum=0.01, maximum=1.0, step=0.01)

t = InstanceBuffer(np.linspace(0, 3.14 * count / 1000, count))

a_wave = 10.0 * vl.cos(a)
b_wave = 100.0 * vl.sin(b)
c_wave = 50.0 * vl.cos(c)
x = vl.cos(a_wave * t) + vl.cos(b_wave * t) / 2.0 + vl.sin(c_wave * t) / 3.0
y = vl.sin(a_wave * t) + vl.sin(b_wave * t) / 2.0 + vl.cos(c_wave * t) / 3.0
z = t
position = 10.0 * vl.vec3(x, y, z)

spheres = Spheres(
    position=position,
    radius=radius,
    color=position / 4.0 + 8.0 / 3.0,
)

Figure().show([spheres])
