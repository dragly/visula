from .expression import Expression


def cos(expr):
    return Expression(Expression(expr).inner.cos())

def sin(expr):
    return Expression(Expression(expr).inner.sin())

def tan(expr):
    return Expression(Expression(expr).inner.tan())
