from typing import TYPE_CHECKING

if TYPE_CHECKING:
    # Avoid circular import
    from .connection import DocConnection

import pycrdt


class UpdateContext:
    """
    Context manager for retrieving a Yjs document from a connection, applying updates
    to it, and then sending the updates to the server.

    Usage:

    ```python
    import pycrdt

    dm = DocumentManager(CONNECTION_STRING)
    conn = dm.get_connection("my-doc")

    with conn.for_update() as doc:
        my_map = doc.get("my-map", pycrdt.Map)
        my_map['foo'] = 'bar'
    ```
    """

    def __init__(self, conn: "DocConnection"):
        self.conn = conn
        self.doc = None
        self.state = None

    def __enter__(self) -> pycrdt.Doc:
        update = self.conn.get_as_update()
        self.doc = pycrdt.Doc()
        self.doc.apply_update(update)
        self.state = self.doc.get_state()
        return self.doc

    def __exit__(self, exc_type, exc_value, traceback):
        if exc_type is not None:
            return False

        update = self.doc.get_update(self.state)
        self.conn.update_doc(update)
