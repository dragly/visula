from ._visula_pyo3 import Application

_app = None

class Visula:
    @staticmethod
    def application():
        global _app
        if _app is None:
            _app = Application()
        return _app
