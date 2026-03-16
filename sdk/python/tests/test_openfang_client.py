import json
import pathlib
import sys
import unittest
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

from openfang_client import OpenFang  # noqa: E402


class _Response:
    def __init__(self, payload, headers=None):
        self._payload = payload
        self.headers = headers or {"content-type": "application/json"}

    def read(self):
        return json.dumps(self._payload).encode()

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        return False


class OpenFangClientTests(unittest.TestCase):
    def test_upload_preserves_custom_headers(self):
        client = OpenFang(
            "http://localhost:4200",
            headers={"Authorization": "Bearer secret-token", "X-Test": "1"},
        )

        captured = {}

        def fake_urlopen(req):
            captured["headers"] = dict(req.header_items())
            captured["method"] = req.get_method()
            captured["url"] = req.full_url
            captured["body"] = req.data
            return _Response(
                {
                    "file_id": "file-1",
                    "filename": "notes.txt",
                    "content_type": "text/plain",
                }
            )

        with mock.patch("openfang_client.urlopen", fake_urlopen):
            result = client.agents.upload(
                "agent-1",
                b"hello world",
                "notes.txt",
                content_type="text/plain",
            )

        self.assertEqual(result["file_id"], "file-1")
        self.assertEqual(
            captured["url"], "http://localhost:4200/api/agents/agent-1/upload"
        )
        self.assertEqual(captured["method"], "POST")
        self.assertEqual(captured["body"], b"hello world")
        self.assertEqual(captured["headers"]["Authorization"], "Bearer secret-token")
        self.assertEqual(captured["headers"]["X-test"], "1")
        self.assertEqual(captured["headers"]["Content-type"], "text/plain")
        self.assertEqual(captured["headers"]["X-filename"], "notes.txt")


if __name__ == "__main__":
    unittest.main()
