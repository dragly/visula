from .lib import PyApplication, PyEventLoop
from dataclasses import dataclass

_visula_singleton = None


@dataclass(frozen=True)
class VisulaSingleton:
    application: PyApplication
    event_loop: PyEventLoop


def _init():
    global _visula_singleton
    if _visula_singleton is None:
        event_loop = PyEventLoop()
        application = PyApplication(event_loop)
        _visula_singleton = VisulaSingleton(
            application=application,
            event_loop=event_loop,
        )


class Visula:
    @staticmethod
    def application():
        _init()
        assert _visula_singleton is not None
        return _visula_singleton.application

    @staticmethod
    def event_loop():
        _init()
        assert _visula_singleton is not None
        return _visula_singleton.event_loop
