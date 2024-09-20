"""Note: These tests require the y-sweet server to be running."""

import unittest
from y_sweet_sdk import DocumentManager, YSweetError
from os import environ
import random
import string
import pycrdt as Y

CONNECTION_STRING = environ.get("CONNECTION_STRING", "ys://localhost:8080")


class TestYSweet(unittest.TestCase):
    def setUp(self):
        self.random_string = "".join(
            random.choices(string.ascii_lowercase + string.digits, k=10)
        )

    def test_create_doc(self):
        doc = DocumentManager(CONNECTION_STRING)
        name = f"{self.random_string}-test-doc"
        result = doc.create_doc(name)
        self.assertEqual(result["docId"], name)

    def test_get_client_token(self):
        doc = DocumentManager(CONNECTION_STRING)

        # getting a non-existent token should raise an error
        nonexistent = f"{self.random_string}-nonexistent"
        with self.assertRaises(YSweetError):
            doc.get_client_token(nonexistent)

        existing = f"{self.random_string}-existing"
        doc.create_doc(existing)
        result = doc.get_client_token(existing)
        self.assertEqual(result["docId"], existing)

    def test_get_url(self):
        doc = DocumentManager(CONNECTION_STRING)
        name = f"{self.random_string}-test-doc"

        doc.get_websocket_url(name)

    def test_get_update(self):
        dm = DocumentManager(CONNECTION_STRING)
        name = f"{self.random_string}-test-doc"
        dm.create_doc(name)
        conn = dm.get_connection(name)

        # Generate an update to apply
        doc = Y.Doc()
        text = doc.get("text", type=Y.Text)
        text.insert(0, "Hello, world!")

        # Get the update
        update = doc.get_update()

        conn.update_doc(update)

        # Get the update from the server
        update = conn.get_as_update()
        doc2 = Y.Doc()
        doc2.apply_update(update)
        text2 = doc2.get("text", type=Y.Text)
        self.assertEqual(text2.to_py(), "Hello, world!")

    def test_get_update_direct(self):
        dm = DocumentManager(CONNECTION_STRING)
        name = f"{self.random_string}-test-doc"
        dm.create_doc(name)

        # Generate an update to apply
        doc = Y.Doc()
        text = doc.get("text", type=Y.Text)
        text.insert(0, "Hello, world!")

        # Get the update
        update = doc.get_update()

        dm.update_doc(name, update)

        # Get the update from the server
        update = dm.get_doc_as_update(name)
        doc2 = Y.Doc()
        doc2.apply_update(update)
        text2 = doc2.get("text", type=Y.Text)
        self.assertEqual(text2.to_py(), "Hello, world!")

    def test_update_context(self):
        dm = DocumentManager(CONNECTION_STRING)
        name = f"{self.random_string}-test-doc"
        dm.create_doc(name)
        conn = dm.get_connection(name)

        with conn.for_update() as doc:
            text = doc.get("text", type=Y.Text)
            text.insert(0, "Hello, world!")

        update = dm.get_doc_as_update(name)
        doc2 = Y.Doc()
        doc2.apply_update(update)
        text2 = doc2.get("text", type=Y.Text)
        self.assertEqual(text2.to_py(), "Hello, world!")

if __name__ == "__main__":
    unittest.main()
