from typing import Literal, Tuple

from ._visula_pyo3 import (
    colormap as _colormap,
    srgb as _srgb,
    srgb_to_linear as _srgb_to_linear,
    srgb_u8 as _srgb_u8,
)
from .expression import Expression, ExpressionLike, _ensure_expression

ColormapName = Literal["viridis", "plasma", "magma", "inferno"]


def colormap(value: ExpressionLike, name: ColormapName = "viridis") -> Expression:
    return Expression(_colormap(_ensure_expression(value), name))


def srgb(hex: int) -> Tuple[float, float, float]:
    """Converts an sRGB hex color, like 0x6E8FC9, to the linear values that Visula expects."""
    return _srgb(hex)


def srgb_u8(r: int, g: int, b: int) -> Tuple[float, float, float]:
    """Converts 8-bit sRGB channels to the linear values that Visula expects."""
    return _srgb_u8(r, g, b)


def srgb_to_linear(value: ExpressionLike) -> Expression:
    """Converts an sRGB-encoded vec3 expression to linear values on the GPU."""
    return Expression(_srgb_to_linear(_ensure_expression(value)))


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
