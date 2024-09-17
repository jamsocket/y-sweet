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

## Installation

For development installation with test dependencies:

```bash
pip install -e ".[dev]"
```

## Tests

First run a y-sweet server:

```bash
npx y-sweet serve
```

Then run the tests:

```bash
pytest
```

## Development

Run `ruff format` to format before committing changes.
