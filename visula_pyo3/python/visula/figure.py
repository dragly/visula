from ._visula_pyo3 import Application, spawn, Expression, Points, Spheres
from .application import Visula

class Figure:
    def show(self, spheres, points):
        app = Visula.application()
        spawn(app, spheres, points)
