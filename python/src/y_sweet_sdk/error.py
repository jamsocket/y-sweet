from typing import Dict, Union


class YSweetError(Exception):
    def __init__(self, cause: Dict[str, Union[str, int]]):
        self.cause = cause
        super().__init__(self.get_message(cause))

    @staticmethod
    def get_message(payload: Dict[str, Union[str, int]]) -> str:
        code = payload["code"]
        if code == "ServerRefused":
            return f"Server refused connection. URL: {payload['url']}"
        elif code == "ServerError":
            return f"Server responded with {payload['status']} {payload['message']}. URL: {payload['url']}"
        elif code == "NoAuthProvided":
            return "No auth provided"
        elif code == "InvalidAuthProvided":
            return "Invalid auth provided"
        else:
            return payload["message"]
