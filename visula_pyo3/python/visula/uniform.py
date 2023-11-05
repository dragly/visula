from dataclasses import dataclass, fields
import sys
from typing import Type

from .application import Visula
from .expression import Expression
from ._visula_pyo3 import PyUniformBuffer, PyUniformField
import numpy as np


@dataclass
class Uniform:
    def __init__(self):
        total_size = 0
        uniform_fields= []
        for field in fields(self):
            size = np.dtype(field.type).itemsize
            total_size += size
            uniform_fields.append(PyUniformField(name=field.name, ty="float", size=size))

        self._inner = PyUniformBuffer(Visula.application(), uniform_fields, type(self).__name__)
        self._size = total_size
        self._buffer = np.zeros(self._size, dtype=np.uint8)

    def instance(self):
        new_fields = {field.name: self._inner.field(index) for index, field in enumerate(fields(self))}
        result = type("UniformInstance", (object,), new_fields)()
        # TODO: Create instance on Rust side that
        # includes the relevant code to generate the shader
        return result

    def update(self):
        offset = 0
        for field in fields(self):
            value = getattr(self, field.name)
            size = np.dtype(field.type).itemsize
            self._buffer[offset : (offset + size)] = np.frombuffer(field.type(value).tobytes(), dtype=np.uint8)
            offset += np.dtype(field.type).itemsize

        self._inner.update(Visula.application(), buffer=self._buffer)
