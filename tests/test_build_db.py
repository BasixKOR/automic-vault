import importlib.util
import json
import pathlib
import urllib.error
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "build-db.py"


def load_build_db():
    spec = importlib.util.spec_from_file_location("build_db", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module._GHCR_TOKENS.clear()
    return module


class FakeResponse:
    def __init__(self, payload):
        self.payload = payload

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, traceback):
        return False

    def read(self):
        return json.dumps(self.payload).encode("utf-8")


class GhcrTokenTests(unittest.TestCase):
    def test_ghcr_bearer_token_supports_anonymous_public_pull(self):
        build_db = load_build_db()
        requests = []

        def urlopen(request, timeout=None):
            requests.append(request)
            return FakeResponse({"token": "anonymous-token", "expires_in": 300})

        with mock.patch.object(build_db.urllib.request, "urlopen", urlopen):
            token = build_db._ghcr_bearer_token("homebrew/core/dvdauthor", None)

        self.assertEqual(token, "anonymous-token")
        self.assertIsNone(requests[0].get_header("Authorization"))

    def test_ghcr_bearer_token_falls_back_to_anonymous_pull(self):
        build_db = load_build_db()
        requests = []

        def urlopen(request, timeout=None):
            requests.append(request)
            if len(requests) == 1:
                raise urllib.error.HTTPError(
                    request.full_url,
                    401,
                    "Unauthorized",
                    hdrs=None,
                    fp=None,
                )
            return FakeResponse({"token": "anonymous-token", "expires_in": 300})

        with mock.patch.object(build_db.urllib.request, "urlopen", urlopen):
            token = build_db._ghcr_bearer_token(
                "homebrew/core/dvdauthor",
                "bad-token",
            )

        self.assertEqual(token, "anonymous-token")
        self.assertIsNotNone(requests[0].get_header("Authorization"))
        self.assertIsNone(requests[1].get_header("Authorization"))


if __name__ == "__main__":
    unittest.main()
