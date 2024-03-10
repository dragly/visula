try:
    from IPython import get_ipython

    if not "IPKernelApp" in get_ipython().config:
        raise RuntimeError("IPython not found")

    from .wasm_wrapper import (
        convert,
        vec3,
        Expression,
        LineDelegate,
        show,
        PyInstanceBuffer,
        PyApplication,
        PyEventLoop,
    )

except:
    from ._visula_pyo3 import (
        convert,
        vec3,
        Expression,
        LineDelegate,
        show,
        PyInstanceBuffer,
        PyApplication,
        PyEventLoop,
    )
