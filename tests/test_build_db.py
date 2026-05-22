import importlib.util
import io
import json
import os
import pathlib
import tempfile
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
    def __init__(self, payload, headers=None):
        self.payload = payload
        self.headers = headers or {}

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, traceback):
        return False

    def read(self):
        return json.dumps(self.payload).encode("utf-8")


def http_error(url, code, headers=None):
    return urllib.error.HTTPError(url, code, "error", headers or {}, fp=io.BytesIO())


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


class NpmFetchTests(unittest.TestCase):
    def test_npm_fetch_retries_429_retry_after(self):
        with mock.patch.dict(
            os.environ,
            {
                "NPM_REGISTRY_RPS": "1000",
                "NPM_MAX_RETRIES": "1",
                "NPM_RATE_LIMIT_BUDGET_SECONDS": "10",
            },
        ):
            build_db = load_build_db()
        requests = []
        sleeps = []

        def urlopen(request, timeout=None):
            requests.append(request)
            if len(requests) == 1:
                raise http_error(
                    request.full_url,
                    429,
                    {"Retry-After": "2"},
                )
            return FakeResponse({"ok": True})

        with (
            mock.patch.object(build_db.urllib.request, "urlopen", urlopen),
            mock.patch.object(build_db.time, "sleep", lambda seconds: sleeps.append(seconds)),
        ):
            payload = build_db._npm_fetch_json(
                "https://registry.npmjs.org/example",
                use_cache=False,
            )

        self.assertEqual(payload, {"ok": True})
        self.assertEqual(len(requests), 2)
        self.assertEqual(sleeps[0], 2.0)
        self.assertEqual(build_db._NPM_STATS["rate_limits"], 1)

    def test_npm_fetch_uses_cached_payload_after_retry_budget_exhaustion(self):
        with tempfile.TemporaryDirectory() as tmp:
            with mock.patch.dict(
                os.environ,
                {
                    "NPM_REGISTRY_RPS": "1000",
                    "NPM_MAX_RETRIES": "1",
                    "NPM_RATE_LIMIT_BUDGET_SECONDS": "0.01",
                },
            ):
                build_db = load_build_db()
            build_db.CACHE_DIR = tmp
            build_db.FORCE_REFRESH = True
            url = "https://registry.npmjs.org/example"
            path = build_db._cache_path_for(url, build_db.NPM_ECOSYSTEM)
            build_db._write_cache(path, {"cached": True}, None, 0)

            def urlopen(request, timeout=None):
                raise http_error(
                    request.full_url,
                    429,
                    {"Retry-After": "60"},
                )

            with mock.patch.object(build_db.urllib.request, "urlopen", urlopen):
                payload = build_db._npm_fetch_json(url)

        self.assertEqual(payload, {"cached": True})
        self.assertEqual(build_db._NPM_STATS["stale_uses"], 1)

    def test_npm_fetch_fails_without_cache_after_retry_budget_exhaustion(self):
        with tempfile.TemporaryDirectory() as tmp:
            with mock.patch.dict(
                os.environ,
                {
                    "NPM_REGISTRY_RPS": "1000",
                    "NPM_MAX_RETRIES": "1",
                    "NPM_RATE_LIMIT_BUDGET_SECONDS": "0.01",
                },
            ):
                build_db = load_build_db()
            build_db.CACHE_DIR = tmp

            def urlopen(request, timeout=None):
                raise http_error(
                    request.full_url,
                    429,
                    {"Retry-After": "60"},
                )

            with mock.patch.object(build_db.urllib.request, "urlopen", urlopen):
                with self.assertRaises(build_db.NpmRateLimitExceeded):
                    build_db._npm_fetch_json("https://registry.npmjs.org/example")

    def test_npm_auth_header_is_limited_to_npm_hosts(self):
        with mock.patch.dict(os.environ, {"NPM_TOKEN": "secret-token"}):
            build_db = load_build_db()
        requests = []

        def urlopen(request, timeout=None):
            requests.append(request)
            return FakeResponse({"ok": True})

        with (
            mock.patch.dict(os.environ, {"NPM_TOKEN": "secret-token"}),
            mock.patch.object(build_db.urllib.request, "urlopen", urlopen),
        ):
            build_db._npm_fetch_json(
                "https://registry.npmjs.org/example",
                use_cache=False,
            )

        self.assertEqual(
            dict(requests[0].header_items()).get("Authorization"),
            "Bearer secret-token",
        )
        with self.assertRaises(ValueError):
            build_db._npm_fetch_json("https://example.com/example", use_cache=False)

    def test_npm_download_batch_keeps_stale_counts_when_rate_limited(self):
        build_db = load_build_db()
        existing = {
            "example": {
                "popularity": {
                    "downloads_per_30_days": 123456,
                }
            }
        }

        with mock.patch.object(
            build_db,
            "_npm_fetch_json",
            side_effect=build_db.NpmRateLimitExceeded("rate limited"),
        ):
            downloads = build_db._npm_monthly_downloads_batch(["example"], existing)

        self.assertEqual(downloads, {"example": 123456})

    def test_npm_download_batch_falls_back_to_single_package_requests(self):
        build_db = load_build_db()
        urls = []

        def fetch(url, accept="application/json", use_cache=True):
            urls.append(url)
            if "," in url:
                raise build_db.NpmTransientFetchError("bad batch")
            return {"downloads": 654321}

        with mock.patch.object(build_db, "_npm_fetch_json", fetch):
            downloads = build_db._npm_monthly_downloads_batch(
                ["@scope/example", "plain"],
                {},
            )

        self.assertEqual(downloads, {"@scope/example": 654321, "plain": 654321})
        self.assertEqual(len(urls), 3)


class NpmIndexTests(unittest.TestCase):
    def test_npm_single_bin_object_can_supply_executable_name(self):
        build_db = load_build_db()
        self.assertEqual(
            build_db._npm_matching_executable(
                "@scope/package",
                {"custom-cli": "bin/custom.js"},
            ),
            "custom-cli",
        )
        self.assertIsNone(
            build_db._npm_matching_executable(
                "package",
                {"one": "bin/one.js", "two": "bin/two.js"},
            )
        )
        self.assertIsNone(
            build_db._npm_matching_executable("package", {"bad/name": "bin.js"})
        )

    def test_npm_full_scan_resumes_from_saved_cursor(self):
        with tempfile.TemporaryDirectory() as tmp:
            build_db = load_build_db()
            build_db.NPM_INDEX_STATE_PATH = os.path.join(tmp, "index.json")
            build_db.NPM_FULL_SCAN_PAGE_SIZE = 2
            state = build_db._default_npm_index_state()
            state["full_scan_cursor"] = "left-off"
            state["packages"] = {}
            urls = []

            def fetch(url, accept="application/json", use_cache=True):
                urls.append(url)
                if "_all_docs" in url:
                    return {
                        "rows": [
                            {
                                "id": "example-cli",
                                "doc": {
                                    "name": "example-cli",
                                    "dist-tags": {"latest": "1.0.0"},
                                    "versions": {
                                        "1.0.0": {
                                            "bin": {"example-cli": "bin.js"},
                                            "description": "example",
                                        }
                                    },
                                    "time": {"1.0.0": "2026-01-01T00:00:00.000Z"},
                                },
                            }
                        ]
                    }
                if "downloads/point" in url:
                    return {"example-cli": {"downloads": 60000}}
                raise AssertionError(url)

            with mock.patch.object(build_db, "_npm_fetch_json", fetch):
                build_db._run_npm_full_scan(state)

            self.assertTrue(any("startkey=%22left-off%22" in url for url in urls))
            self.assertIsNone(state["full_scan_cursor"])
            self.assertIn("example-cli", state["packages"])

    def test_npm_collect_fails_without_usable_cache_on_rate_limit(self):
        build_db = load_build_db()
        state = build_db._default_npm_index_state()

        with (
            mock.patch.object(build_db, "_read_npm_index_state", return_value=state),
            mock.patch.object(
                build_db,
                "_run_npm_full_scan",
                side_effect=build_db.NpmRateLimitExceeded("rate limited"),
            ),
        ):
            with self.assertRaises(build_db.NpmRateLimitExceeded):
                build_db._collect_npm_metadata()

    def test_npm_collect_preserves_cached_packages_on_rate_limit(self):
        build_db = load_build_db()
        state = build_db._default_npm_index_state()
        state["last_seq"] = "10"
        state["packages"] = {
            "example-cli": {
                "executable": "example-cli",
                "homepage": "",
                "last_updated_at": "2026-01-01T00:00:00.000Z",
                "popularity": {"downloads_per_30_days": 60000, "rank": 99},
                "summary": "example",
                "version": "1.0.0",
            }
        }

        with (
            mock.patch.object(build_db, "_read_npm_index_state", return_value=state),
            mock.patch.object(
                build_db,
                "_fetch_npm_changes_since",
                side_effect=build_db.NpmRateLimitExceeded("rate limited"),
            ),
            mock.patch.object(build_db, "_write_npm_index_state"),
        ):
            packages = build_db._collect_npm_metadata()

        self.assertEqual(list(packages), ["example-cli"])
        self.assertEqual(packages["example-cli"]["popularity"]["rank"], 1)

    def test_npm_changes_stop_at_refresh_limit_and_return_processed_sequence(self):
        build_db = load_build_db()

        def fetch(url, accept="application/json", use_cache=True):
            return {
                "last_seq": "page-end",
                "results": [
                    {"id": "one", "seq": "11"},
                    {"id": "two", "seq": "12"},
                    {"id": "three", "seq": "13"},
                ],
            }

        with mock.patch.object(build_db, "_npm_fetch_json", fetch):
            changed, deleted, next_seq, has_more = build_db._fetch_npm_changes_since(
                "10",
                max_changes=2,
            )

        self.assertEqual(changed, {"one", "two"})
        self.assertEqual(deleted, set())
        self.assertEqual(next_seq, "12")
        self.assertTrue(has_more)


if __name__ == "__main__":
    unittest.main()
