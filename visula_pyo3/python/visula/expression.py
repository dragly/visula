from __future__ import annotations
from .application import Visula
from ._visula_pyo3 import convert, vec3 as _vec3, Expression as _Expression


def _ensure_expression(other):
    if isinstance(other, Expression):
        return other.inner
    else:
        return convert(Visula.application(), other)


class Expression:
    inner: _Expression

    def __init__(self, obj):
        self.inner = convert(Visula.application(), obj)

    def __add__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.add(o))

    def __radd__(self, other) -> Expression:
        return self + other

    def __sub__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.sub(o))

    def __mul__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.mul(o))

    def __rmul__(self, other) -> Expression:
        return self + other

    def __truediv__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.truediv(o))

    def __floordiv__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.floordiv(o))

    def __mod__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.mod(o))

    def __pow__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.pow(o))

def vec3(x, y, z):
    x = _ensure_expression(x)
    y = _ensure_expression(y)
    z = _ensure_expression(z)
    return Expression(_vec3(x, y, z))
