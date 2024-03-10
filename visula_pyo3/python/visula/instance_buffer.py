from .application import Visula
from .expression import Expression
from .lib import PyInstanceBuffer


class InstanceBuffer(Expression):
    inner_buffer: PyInstanceBuffer

    def __init__(self, obj):
        self.inner_buffer = PyInstanceBuffer(pyapplication=Visula.application(), obj=obj)
        self.inner = self.inner_buffer.instance()

    def update(self, data):
        application = Visula.application()
        self.inner_buffer.update_buffer(pyapplication=application, data=data)
