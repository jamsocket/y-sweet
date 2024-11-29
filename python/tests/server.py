import subprocess
import signal
from typing import Optional
from os import path, environ
import json
import time
import re


BASE_DIR = path.join(path.dirname(__file__), "..", "..", "crates", "y-sweet")


class Server:
    @staticmethod
    def build():
        subprocess.run(
            ["cargo", "build"],
            cwd=BASE_DIR,
            check=True,
        )

    def __init__(self, test_id: str):
        self.data_dir = path.join(path.dirname(__file__), "..", "test-out", test_id)
        self.process = subprocess.Popen(
            ["cargo", "run", "--", "serve", self.data_dir, "--port", "0"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=BASE_DIR,
            env={"Y_SWEET_LOG_JSON": "true", **environ},
            text=True,
            bufsize=1,
        )

        # Wait for connection string in the output
        while self.process.poll() is None:
            line = self.process.stdout.readline()
            print("here", line)
            match = re.search(r"CONNECTION_STRING=([^ ]+)", line)
            if match:
                self.connection_string = match.group(1)
                break

        if not self.connection_string:
            raise RuntimeError("Server failed to start and provide connection string")

    def shutdown(self):
        if self.process:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()

            # Clean up the process
            self.process = None
