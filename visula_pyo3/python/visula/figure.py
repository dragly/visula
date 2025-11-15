from typing import Any, Optional, Sequence

from ._visula_pyo3 import show
from .application import Visula
from .gui import Slider


class Figure:
    def show(
        self,
        renderables: Sequence[Any],
        update,
        controls: Optional[Sequence[Slider]] = None,
    ):
        controls = controls or []
        app = Visula.application()
        event_loop = Visula.event_loop()
        show(
            py_application=app,
            py_renderables=renderables,
            update=update,
            controls=controls,
        )
        event_loop.run(app)
