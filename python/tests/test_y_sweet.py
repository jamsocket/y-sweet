"""Note: These tests require the y-sweet server to be running."""

import unittest
from y_sweet_sdk import DocumentManager, YSweetError
from os import environ
import random
import string

CONNECTION_STRING = environ.get('CONNECTION_STRING', 'ys://localhost:8080')

class TestYSweet(unittest.TestCase):
    def setUp(self):
        self.random_string = ''.join(random.choices(string.ascii_lowercase + string.digits, k=10))

    def test_create_doc(self):
        doc = DocumentManager(CONNECTION_STRING)
        name = f'{self.random_string}-test-doc'
        result = doc.create_doc(name)
        self.assertEqual(result['docId'], name)

    def test_get_client_token(self):
        doc = DocumentManager(CONNECTION_STRING)
        
        # getting a non-existent token should raise an error
        nonexistant = f'{self.random_string}-nonexistent'
        with self.assertRaises(YSweetError):
            doc.get_client_token(nonexistant)
        
        existing = f'{self.random_string}-existing'
        doc.create_doc(existing)
        result = doc.get_client_token(existing)
        self.assertEqual(result['docId'], existing)

    def test_get_url(self):
        doc = DocumentManager(CONNECTION_STRING)
        name = f'{self.random_string}-test-doc'
        
        doc.get_websocket_url(name)

if __name__ == '__main__':
    unittest.main()
