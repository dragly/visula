from typing import Literal

from ._visula_pyo3 import colormap as _colormap
from .expression import Expression, ExpressionLike, _ensure_expression

ColormapName = Literal["viridis", "plasma", "magma", "inferno"]


def colormap(value: ExpressionLike, name: ColormapName = "viridis") -> Expression:
    return Expression(_colormap(_ensure_expression(value), name))


def cos(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).cos())


def sin(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).sin())


def tan(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).tan())


def sqrt(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).sqrt())


def abs(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).abs())


def exp(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).exp())


def log(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).log())


def floor(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).floor())


def ceil(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).ceil())


def round(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).round())


def fract(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).fract())


def sign(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).sign())


def length(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).length())


def normalize(value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).normalize())


def min(a: ExpressionLike, b: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(a).min(_ensure_expression(b)))


def max(a: ExpressionLike, b: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(a).max(_ensure_expression(b)))


def dot(a: ExpressionLike, b: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(a).dot(_ensure_expression(b)))


def cross(a: ExpressionLike, b: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(a).cross(_ensure_expression(b)))


def distance(a: ExpressionLike, b: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(a).distance(_ensure_expression(b)))


def atan2(y: ExpressionLike, x: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(y).atan2(_ensure_expression(x)))


def pow(base: ExpressionLike, exponent: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(base).pow(_ensure_expression(exponent)))


def clamp(value: ExpressionLike, low: ExpressionLike, high: ExpressionLike) -> Expression:
    return Expression(
        _ensure_expression(value).clamp(_ensure_expression(low), _ensure_expression(high))
    )


def mix(a: ExpressionLike, b: ExpressionLike, amount: ExpressionLike) -> Expression:
    return Expression(
        _ensure_expression(a).mix(_ensure_expression(b), _ensure_expression(amount))
    )


def smoothstep(
    edge_low: ExpressionLike, edge_high: ExpressionLike, value: ExpressionLike
) -> Expression:
    return Expression(
        _ensure_expression(value).smoothstep(
            _ensure_expression(edge_low), _ensure_expression(edge_high)
        )
    )


def step(edge: ExpressionLike, value: ExpressionLike) -> Expression:
    return Expression(_ensure_expression(value).step(_ensure_expression(edge)))
