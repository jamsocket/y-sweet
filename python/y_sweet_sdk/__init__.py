import json
import random
import string
from typing import Dict, Union, Optional
from urllib.parse import urlparse
import requests

class YSweetError(Exception):
    def __init__(self, cause: Dict[str, Union[str, int]]):
        self.cause = cause
        super().__init__(self.get_message(cause))

    @staticmethod
    def get_message(payload: Dict[str, Union[str, int]]) -> str:
        code = payload['code']
        if code == 'ServerRefused':
            return f"Server refused connection. URL: {payload['url']}"
        elif code == 'ServerError':
            return f"Server responded with {payload['status']} {payload['message']}. URL: {payload['url']}"
        elif code == 'NoAuthProvided':
            return "No auth provided"
        elif code == 'InvalidAuthProvided':
            return "Invalid auth provided"
        else:
            return payload['message']


class DocumentManager:
    def __init__(self, connection_string: str):
        parsed_url = urlparse(connection_string)
        self.token = parsed_url.username and requests.utils.unquote(parsed_url.username)
        
        protocol = 'http' if parsed_url.scheme == 'ys' else 'https'
        self.base_url = f"{protocol}://{parsed_url.netloc}{parsed_url.path}".rstrip('/')

    def _do_request(self, path: str, method: str = 'GET', data: Optional[Dict] = None) -> requests.Response:
        url = f"{self.base_url}/{path}"
        headers = {'Authorization': f"Bearer {self.token}"} if self.token else {}

        try:
            response = requests.request(
                method,
                url,
                headers=headers,
                json=data,
                params={'z': ''.join(random.choices(string.ascii_lowercase + string.digits, k=8))}
            )
            response.raise_for_status()
        except requests.RequestException as e:
            if isinstance(e, requests.ConnectionError):
                raise YSweetError({'code': 'ServerRefused', 'url': url}) from e
            elif isinstance(e, requests.HTTPError):
                if e.response.status_code == 401:
                    if self.token:
                        raise YSweetError({'code': 'InvalidAuthProvided'}) from e
                    else:
                        raise YSweetError({'code': 'NoAuthProvided'}) from e
                elif e.response.status_code == 404:
                    raise YSweetError({'code': 'NotFound', 'status': e.response.status_code, 'message': e.response.reason, 'url': url}) from e
                raise YSweetError({'code': 'ServerError', 'status': e.response.status_code, 'message': e.response.reason, 'url': url}) from e
            else:
                raise YSweetError({'code': 'Unknown', 'message': str(e)}) from e
        except Exception as e:
            raise YSweetError({'code': 'Unknown', 'message': str(e)}) from e
        
        return response

    def check_store(self) -> Dict[str, Union[bool, str]]:
        return self._do_request('check_store').json()

    def create_doc(self, doc_id: Optional[str] = None) -> Dict[str, str]:
        data = {'docId': doc_id} if doc_id else {}
        return self._do_request('doc/new', method='POST', data=data).json()

    def get_client_token(self, doc_id: Union[str, Dict[str, str]]) -> Dict[str, str]:
        if isinstance(doc_id, dict):
            doc_id = doc_id['docId']
        return self._do_request(f"doc/{doc_id}/auth", method='POST', data={}).json()

    def get_or_create_doc_and_token(self, doc_id: Optional[str] = None) -> Dict[str, str]:
        result = self.create_doc(doc_id)
        return self.get_client_token(result)

    def get_doc_as_update(self, doc_id: str) -> bytes:
        return self._do_request(f"doc/{doc_id}/as-update").content

    def update_doc(self, doc_id: str, update: bytes) -> None:
        url = f"{self.base_url}/doc/{doc_id}/update"
        headers = {'Content-Type': 'application/octet-stream'}
        if self.token:
            headers['Authorization'] = f"Bearer {self.token}"
        response = requests.post(url, data=update, headers=headers)
        response.raise_for_status()

    def get_websocket_url(self, doc_id: str) -> str:
        self.create_doc(doc_id)
        conn = self.get_client_token(doc_id)

        url = conn["url"]
        token = conn.get("token")
        doc_id = conn["docId"]

        if token:
            return f"{url}/{doc_id}?token={token}"
        else:
            return f"{url}/{doc_id}"

def get_or_create_doc_and_token(connection_string: str, doc_id: Optional[str] = None) -> Dict[str, str]:
    manager = DocumentManager(connection_string)
    return manager.get_or_create_doc_and_token(doc_id)

def get_client_token(connection_string: str, doc_id: Union[str, Dict[str, str]]) -> Dict[str, str]:
    manager = DocumentManager(connection_string)
    return manager.get_client_token(doc_id)

def create_doc(connection_string: str, doc_id: Optional[str] = None) -> Dict[str, str]:
    manager = DocumentManager(connection_string)
    return manager.create_doc(doc_id)

def encode_client_token(token: Dict[str, str]) -> str:
    json_string = json.dumps(token)
    return requests.utils.quote(json_string)

def decode_client_token(token: str) -> Dict[str, str]:
    json_string = requests.utils.unquote(token)
    return json.loads(json_string)
