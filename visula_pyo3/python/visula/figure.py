from typing import Any, Sequence

from ._visula_pyo3 import show
from .application import Visula

class Figure:
    def show(self, renderables: Sequence[Any]):
        app = Visula.application()
        show(app, renderables)
