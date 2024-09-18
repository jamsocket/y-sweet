import requests
from typing import Dict, Optional

from .update import UpdateContext


class DocConnection:
    def __init__(self, client_token: Dict[str, str]):
        self.base_url = client_token["baseUrl"].rstrip("/")
        self.token = client_token.get("token")
        self.doc_id = client_token["docId"]
        self.headers = {"Authorization": f"Bearer {self.token}"} if self.token else {}

    def _do_request(
        self, path: str, method: str = "GET", data: Optional[bytes] = None
    ) -> requests.Response:
        url = f"{self.base_url}/{path}"
        response = requests.request(method, url, headers=self.headers, data=data)
        response.raise_for_status()
        return response

    def get_as_update(self) -> bytes:
        """
        Returns an entire document, represented as a Yjs update byte string.

        Returns:
            bytes: The Yjs update as bytes.
        """
        response = self._do_request("as-update")
        return response.content

    def update_doc(self, update: bytes) -> None:
        """
        Updates a document with the given Yjs update byte string.

        Args:
            update (bytes): The Yjs update as bytes.
        """
        self._do_request("update", method="POST", data=update)

    def for_update(self) -> UpdateContext:
        return UpdateContext(self)
