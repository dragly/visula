from ._visula_pyo3 import colormap as _colormap
from .expression import Expression, _ensure_expression


def colormap(value, name="viridis"):
    return Expression(_colormap(Expression(value).inner, name))


def _unary(name):
    def function(expr):
        return Expression(getattr(Expression(expr).inner, name)())

    function.__name__ = name
    return function


def _binary(name):
    def function(a, b):
        return Expression(getattr(Expression(a).inner, name)(_ensure_expression(b)))

    function.__name__ = name
    return function


cos = _unary("cos")
sin = _unary("sin")
tan = _unary("tan")
sqrt = _unary("sqrt")
abs = _unary("abs")
exp = _unary("exp")
log = _unary("log")
floor = _unary("floor")
ceil = _unary("ceil")
round = _unary("round")
fract = _unary("fract")
sign = _unary("sign")
length = _unary("length")
normalize = _unary("normalize")

min = _binary("min")
max = _binary("max")
dot = _binary("dot")
cross = _binary("cross")
distance = _binary("distance")
atan2 = _binary("atan2")
pow = _binary("pow")


def clamp(value, low, high):
    return Expression(
        Expression(value).inner.clamp(_ensure_expression(low), _ensure_expression(high))
    )


def mix(a, b, amount):
    return Expression(
        Expression(a).inner.mix(_ensure_expression(b), _ensure_expression(amount))
    )


def smoothstep(edge_low, edge_high, value):
    return Expression(
        Expression(value).inner.smoothstep(
            _ensure_expression(edge_low), _ensure_expression(edge_high)
        )
    )


def step(edge, value):
    return Expression(Expression(value).inner.step(_ensure_expression(edge)))
