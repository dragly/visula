from typing import Any, Callable, Literal, Optional, Sequence, Tuple, Union

from ._visula_pyo3 import show
from .application import Visula
from .gui import Slider, all_sliders

TonemappingName = Literal["aces", "reinhard", "none"]


class Figure:
    def __init__(
        self,
        clear_color: Optional[Tuple[float, float, float]] = None,
        tonemapping: TonemappingName = "aces",
    ):
        self.clear_color = clear_color
        self.tonemapping = tonemapping

    def show(
        self,
        renderables: Union[Any, Sequence[Any]],
        update: Optional[Callable[[], None]] = None,
        controls: Optional[Sequence[Slider]] = None,
    ):
        if not isinstance(renderables, (list, tuple)):
            renderables = [renderables]
        if controls is None:
            controls = all_sliders()
        app = Visula.application()
        event_loop = Visula.event_loop()
        show(
            py_application=app,
            py_renderables=list(renderables),
            clear_color=self.clear_color,
            tonemapping=self.tonemapping,
            update=update,
            controls=[control._inner for control in controls],
        )
        event_loop.run(app)
