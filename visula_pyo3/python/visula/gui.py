from ._visula_pyo3 import PySlider
from .application import Visula
from .expression import Expression

_sliders = []


def all_sliders():
    return list(_sliders)


class Slider(Expression):
    def __init__(self, name, value=0.0, minimum=0.0, maximum=1.0, step=0.0):
        self._inner = PySlider(
            Visula.application(),
            name=name,
            value=value,
            minimum=minimum,
            maximum=maximum,
            step=step,
        )
        self.inner = self._inner.expression()
        _sliders.append(self)

    @property
    def value(self):
        return self._inner.value

    @value.setter
    def value(self, value):
        self._inner.value = value


__all__ = ["Slider", "all_sliders"]
