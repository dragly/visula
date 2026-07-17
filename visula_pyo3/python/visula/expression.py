from __future__ import annotations
from .application import Visula
from ._visula_pyo3 import (
    convert,
    vec2 as _vec2,
    vec3 as _vec3,
    vec4 as _vec4,
    Expression as _Expression,
)


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

    def __rsub__(self, other) -> Expression:
        return Expression(other) - self

    def __mul__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.mul(o))

    def __rmul__(self, other) -> Expression:
        return self * other

    def __truediv__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.truediv(o))

    def __rtruediv__(self, other) -> Expression:
        return Expression(other) / self

    def __floordiv__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.floordiv(o))

    def __rfloordiv__(self, other) -> Expression:
        return Expression(other) // self

    def __mod__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.modulo(o))

    def __rmod__(self, other) -> Expression:
        return Expression(other) % self

    def __pow__(self, other) -> Expression:
        o = _ensure_expression(other)
        return Expression(self.inner.pow(o))

    def __rpow__(self, other) -> Expression:
        return Expression(other) ** self

    def __neg__(self) -> Expression:
        return Expression(self.inner.neg())


def vec2(x, y):
    x = _ensure_expression(x)
    y = _ensure_expression(y)
    return Expression(_vec2(x, y))


def vec3(x, y, z):
    x = _ensure_expression(x)
    y = _ensure_expression(y)
    z = _ensure_expression(z)
    return Expression(_vec3(x, y, z))


def vec4(x, y, z, w):
    x = _ensure_expression(x)
    y = _ensure_expression(y)
    z = _ensure_expression(z)
    w = _ensure_expression(w)
    return Expression(_vec4(x, y, z, w))
