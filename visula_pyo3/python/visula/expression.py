from __future__ import annotations

from typing import Union

import numpy.typing as npt

from .application import Visula
from ._visula_pyo3 import (
    convert,
    vec2 as _vec2,
    vec3 as _vec3,
    vec4 as _vec4,
    Expression as _Expression,
)


def _ensure_expression(other: ExpressionLike) -> _Expression:
    if isinstance(other, Expression):
        return other.inner
    else:
        return convert(Visula.application(), other)


class Expression:
    inner: _Expression

    def __init__(self, obj: ExpressionLike):
        if isinstance(obj, _Expression):
            self.inner = obj
        else:
            self.inner = convert(Visula.application(), obj)

    def __add__(self, other: ExpressionLike) -> Expression:
        return Expression(self.inner.add(_ensure_expression(other)))

    def __radd__(self, other: ExpressionLike) -> Expression:
        return self + other

    def __sub__(self, other: ExpressionLike) -> Expression:
        return Expression(self.inner.sub(_ensure_expression(other)))

    def __rsub__(self, other: ExpressionLike) -> Expression:
        return Expression(other) - self

    def __mul__(self, other: ExpressionLike) -> Expression:
        return Expression(self.inner.mul(_ensure_expression(other)))

    def __rmul__(self, other: ExpressionLike) -> Expression:
        return self * other

    def __truediv__(self, other: ExpressionLike) -> Expression:
        return Expression(self.inner.truediv(_ensure_expression(other)))

    def __rtruediv__(self, other: ExpressionLike) -> Expression:
        return Expression(other) / self

    def __floordiv__(self, other: ExpressionLike) -> Expression:
        return Expression(self.inner.floordiv(_ensure_expression(other)))

    def __rfloordiv__(self, other: ExpressionLike) -> Expression:
        return Expression(other) // self

    def __mod__(self, other: ExpressionLike) -> Expression:
        return Expression(self.inner.modulo(_ensure_expression(other)))

    def __rmod__(self, other: ExpressionLike) -> Expression:
        return Expression(other) % self

    def __pow__(self, other: ExpressionLike) -> Expression:
        return Expression(self.inner.pow(_ensure_expression(other)))

    def __rpow__(self, other: ExpressionLike) -> Expression:
        return Expression(other) ** self

    def __neg__(self) -> Expression:
        return Expression(self.inner.neg())


ExpressionLike = Union[Expression, _Expression, npt.ArrayLike]


def vec2(x: ExpressionLike, y: ExpressionLike) -> Expression:
    return Expression(_vec2(_ensure_expression(x), _ensure_expression(y)))


def vec3(x: ExpressionLike, y: ExpressionLike, z: ExpressionLike) -> Expression:
    return Expression(
        _vec3(_ensure_expression(x), _ensure_expression(y), _ensure_expression(z))
    )


def vec4(
    x: ExpressionLike, y: ExpressionLike, z: ExpressionLike, w: ExpressionLike
) -> Expression:
    return Expression(
        _vec4(
            _ensure_expression(x),
            _ensure_expression(y),
            _ensure_expression(z),
            _ensure_expression(w),
        )
    )
