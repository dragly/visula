from ._visula_pyo3 import show
from .application import Visula

class Figure:
    def show(self, spheres):
        app = Visula.application()
        show(app, spheres)
