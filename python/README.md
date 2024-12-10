# y_sweet_sdk

## Usage

```python
from y_sweet_sdk import DocumentManager

# Get the websocket url for a document.
doc = DocumentManager('ys://localhost:8080')
url = doc.get_websocket_url('my-document-id')

# Connect to the document using y_py and ypy_websocket.
# (Based on: https://davidbrochart.github.io/ypy-websocket/usage/client/)
from ypy_websocket import WebsocketProvider
import y_py as Y
from websockets import connect
import asyncio

ydoc = Y.YDoc()

# Simple example: log the array "todolist" to stdout every time it changes.
data = ydoc.get_array("todolist")
def data_changed(event: Y.AfterTransactionEvent):
    print(f"data changed: {data.to_json()}")

data.observe_deep(data_changed)

async with (
    connect(url) as websocket,
    WebsocketProvider(ydoc, websocket),
):
    await asyncio.Future()  # run forever
```

`y_sweet_sdk` is only used to talk directly with the Y-Sweet server to obtain a WebSocket URL to pass to a client.
Use a Yjs client like [ypy-websocket](https://davidbrochart.github.io/ypy-websocket/usage/client/) or [pycrdt](https://github.com/jupyter-server/pycrdt)
in conjunction with `y_sweet_sdk` to access the actual Y.Doc data.

## Developing

Developing `y_sweet_sdk` requires the [`uv`](https://docs.astral.sh/uv/) project manager.

To install it on Mac or Liunux, run:

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

(See [the docs](https://docs.astral.sh/uv/) for other platforms and more information.)

When using `uv`, you do not need to manage a virtual environment yourself. Instead, you interact with
Python using the `uv` command, which automatically picks up the virtual environment from the location.

To set up the virtual environment for development, run:

```bash
uv sync --dev
```

This installs both the regular dependencies and the development dependencies.

### Tests

Once commands are installed in your virtual environment, you can run them with `uv run`.

To run tests, run:

```bash
uv run pytest
```

This runs the `pytest` command in the virtual environment.

### Formatting

Run `uv run ruff format` to format before committing changes.
