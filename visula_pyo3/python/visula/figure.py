from typing import Any, Optional, Sequence

from .gui import Slider
from ._visula_pyo3 import show
from .application import Visula


class Figure:
    def show(self, renderables: Sequence[Any], update, controls: Optional[Sequence[Slider]] = None):
        controls = controls or []
        app = Visula.application()
        show(
            py_application=app,
            renderables=renderables,
            update=update,
            controls=controls,
        )
