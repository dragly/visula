from .application import Visula
from .expression import Expression
from ._visula_pyo3 import PyInstanceBuffer


class InstanceBuffer(Expression):
    inner_buffer: PyInstanceBuffer

    def __init__(self, obj):
        super().__init__(obj)
        self.inner_buffer = PyInstanceBuffer(pyapplication=Visula.application(), obj=obj)

    def update(self, data):
        print("Updating in Python...")
        # application = Visula.application()
        # self.inner_buffer.update_buffer(pyapplication=application, data=data)
        # self.inner_buffer.dummy()
