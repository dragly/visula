from typing import Any, Optional, Sequence

from .gui import Slider
from ._visula_pyo3 import show
from .application import Visula


class Figure:
    def show(self, renderables: Sequence[Any], update, controls: Optional[Sequence[Slider]] = None):
        controls = controls or []
        app = Visula.application()
        event_loop = Visula.event_loop()
        show(
            py_application=app,
            py_event_loop=event_loop,
            renderables=renderables,
            update=update,
            controls=controls,
        )
