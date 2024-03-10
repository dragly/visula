from typing import Any, Sequence
from .lib import show
from .application import Visula
from .jupyter import is_running_in_jupyter


class Figure:
    def show(self, renderables: Sequence[Any], update):
        app = Visula.application()
        event_loop = Visula.event_loop()

        if is_running_in_jupyter():
            import base64
            from IPython.display import HTML, display, Javascript

            # Step 1: Serialize Binary Data in Python
            binary_data = b"\x00\x01\x02\x03"  # Replace with your binary data
            base64_encoded_data = base64.b64encode(binary_data).decode("utf-8")

            # Step 2: Create JavaScript Code
            javascript_code = f"""
            // Parse the base64 encoded binary data
            var base64EncodedData = '{base64_encoded_data}';
            var binaryData = atob(base64EncodedData);
            var byteArray = new Uint8Array(binaryData.length);
            for (var i = 0; i < binaryData.length; i++) {{
              byteArray[i] = binaryData.charCodeAt(i);
            }}

            // Use the byteArray for further processing
            // Example: Display the length of the binary data
            console.log('Length of binary data:', byteArray.length);
            """

            display(Javascript(javascript_code))
        else:
            show(py_application=app, py_event_loop=event_loop, renderables=renderables, update=update)
