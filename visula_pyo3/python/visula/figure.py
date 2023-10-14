from typing import Sequence

from visula.renderable import Renderable
from ._visula_pyo3 import show
from .application import Visula

class Figure:
    def show(self, renderables: Sequence[Renderable]):
        app = Visula.application()
        show(app, renderables)
