import requests
from os import environ
from urllib.parse import urlparse
import argparse
from ypy_websocket import WebsocketProvider
import y_py as Y
from websockets import connect
import asyncio

CONNECTION_STRING = environ.get("CONNECTION_STRING")

if not CONNECTION_STRING:
    raise ValueError("CONNECTION_STRING is not set")


class YSweetManager:
    def __init__(self, connection_string: str):
        self.connection_string = connection_string
        
        # the connection string has a password component, which we need to extract with urlparse
        parsed_url = urlparse(self.connection_string)
        # nb: we do not use username@password syntax, so the entire bearer token is the username.
        self.password = parsed_url.username

        scheme = 'https' if parsed_url.scheme == 'yss' else 'http'
        self.url = f"{scheme}://{parsed_url.hostname}{parsed_url.path}"

    def connect(self, doc_id: str):
        url = f"{self.url}doc/{doc_id}/auth"
        response = requests.post(url, headers={"Authorization": f"Bearer {self.password}"}, json={})
        if response.status_code != 200:
            print(response.text())
            raise Exception(f"Failed to connect to {url}")
        return response.json()


async def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("doc_id", type=str, help="The id of the document to connect to")
    args = parser.parse_args()

    y_sweet_manager = YSweetManager(CONNECTION_STRING)

    conn = y_sweet_manager.connect(args.doc_id)

    url = conn["url"]
    token = conn["token"]
    doc_id = conn["docId"]

    full_url = f"{url}/{doc_id}?token={token}"

    ydoc = Y.YDoc()
    data = ydoc.get_array("todolist")

    def data_changed(event: Y.AfterTransactionEvent):
        print(f"data changed: {data.to_json()}")

    data.observe_deep(data_changed)

    async with (
        connect(full_url) as websocket,
        WebsocketProvider(ydoc, websocket),
    ):
        await asyncio.Future()  # run forever



asyncio.run(main())

