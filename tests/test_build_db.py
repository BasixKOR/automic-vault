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
        self.assertEqual(len(urls), 2)
        self.assertFalse(any("," in url for url in urls))

    def test_npm_download_batch_can_skip_single_package_fallback(self):
        build_db = load_build_db()
        urls = []

        def fetch(url, accept="application/json", use_cache=True):
            urls.append(url)
            raise build_db.NpmTransientFetchError("no batch")

        with mock.patch.object(build_db, "_npm_fetch_json", fetch):
            downloads = build_db._npm_monthly_downloads_batch(
                ["@scope/example", "plain"],
                {},
                allow_single_fallback=False,
            )

        self.assertEqual(downloads, {})
        self.assertEqual(
            urls,
            [build_db._npm_downloads_batch_url(["plain"])],
        )

    def test_npm_download_batches_split_by_count_and_url_length(self):
        build_db = load_build_db()
        build_db.NPM_DOWNLOADS_BATCH_SIZE = 3
        first_two_url = build_db._npm_downloads_batch_url(["@scope/example", "plain"])
        build_db.NPM_DOWNLOADS_BATCH_URL_MAX_LENGTH = len(first_two_url)

        batches = list(
            build_db._npm_download_batches(
                ["@scope/example", "plain", "third", "fourth"]
            )
        )

        self.assertEqual(
            batches,
            [["@scope/example", "plain"], ["third", "fourth"]],
        )

        build_db.NPM_DOWNLOADS_BATCH_URL_MAX_LENGTH = 10_000
        batches = list(
            build_db._npm_download_batches(
                ["@scope/example", "plain", "third", "fourth"]
            )
        )

        self.assertEqual(
            batches,
            [["@scope/example", "plain", "third"], ["fourth"]],
        )


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
            build_db.NPM_FULL_SCAN_PAGE_SIZE = 3
            state = build_db._default_npm_index_state()
            state["full_scan_cursor"] = "left-off"
            state["packages"] = {}
            urls = []

            def fetch(url, accept="application/json", use_cache=True):
                urls.append(url)
                if "_all_docs" in url:
                    return {
                        "total_rows": 4,
                        "rows": [
                            {
                                "id": "example-cli",
                            },
                            {
                                "id": "tiny-cli",
                            }
                        ]
                    }
                if "registry.npmjs.org/example-cli" in url:
                    return {
                        "name": "example-cli",
                        "dist-tags": {"latest": "1.0.0"},
                        "versions": {
                            "1.0.0": {
                                "bin": {"example-cli": "bin.js"},
                                "description": "example",
                            }
                        },
                        "time": {"1.0.0": "2026-01-01T00:00:00.000Z"},
                    }
                if "registry.npmjs.org/tiny-cli" in url:
                    raise AssertionError("low-download packages should not fetch packuments")
                if "downloads/point" in url:
                    return {
                        "example-cli": {"downloads": 60000},
                        "tiny-cli": {"downloads": 100},
                    }
                raise AssertionError(url)

            with mock.patch.object(build_db, "_npm_fetch_json", fetch):
                build_db._run_npm_full_scan(state)

            self.assertTrue(any("startkey=%22left-off%22" in url for url in urls))
            self.assertFalse(any("skip=" in url for url in urls))
            self.assertIsNone(state["full_scan_cursor"])
            self.assertIsNone(state["full_scan_started_at"])
            self.assertIsNotNone(state["last_full_scan_at"])
            self.assertEqual(state["full_scan_seen_count"], 2)
            self.assertEqual(state["full_scan_download_qualified_count"], 1)
            self.assertEqual(state["full_scan_packument_qualified_count"], 1)
            self.assertEqual(state["full_scan_page_count"], 1)
            self.assertEqual(state["full_scan_total_rows"], 4)
            self.assertIn("example-cli", state["packages"])

    def test_npm_read_state_clears_stale_started_marker_after_completed_scan(self):
        with tempfile.TemporaryDirectory() as tmp:
            build_db = load_build_db()
            build_db.NPM_INDEX_STATE_PATH = os.path.join(tmp, "index.json")
            os.makedirs(tmp, exist_ok=True)
            with open(build_db.NPM_INDEX_STATE_PATH, "w", encoding="utf-8") as handle:
                json.dump(
                    {
                        "full_scan_cursor": None,
                        "full_scan_started_at": "2026-05-27T00:06:25+00:00",
                        "last_full_scan_at": "2026-06-01T15:18:26+00:00",
                        "packages": {},
                    },
                    handle,
                )

            state = build_db._read_npm_index_state()

            self.assertIsNone(state["full_scan_started_at"])
            self.assertEqual(state["full_scan_seen_count"], 0)
            self.assertEqual(state["full_scan_download_qualified_count"], 0)
            self.assertEqual(state["full_scan_packument_qualified_count"], 0)
            self.assertEqual(state["full_scan_page_count"], 0)
            self.assertIsNone(state["full_scan_total_rows"])

    def test_npm_collect_fails_without_usable_cache_on_rate_limit(self):
        build_db = load_build_db()
        state = build_db._default_npm_index_state()

        with (
            mock.patch.object(build_db, "_read_npm_index_state", return_value=state),
            mock.patch.object(
                build_db,
                "_current_npm_changes_sequence",
                return_value="scan-start",
            ),
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
        state["last_full_scan_at"] = "2026-01-01T00:00:00+00:00"
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

    def test_npm_collect_runs_full_scan_until_one_has_completed(self):
        build_db = load_build_db()
        state = build_db._default_npm_index_state()
        state["last_seq"] = "10"
        state["packages"] = {
            "seed-cli": {
                "executable": "seed-cli",
                "homepage": "",
                "last_updated_at": "2026-01-01T00:00:00.000Z",
                "popularity": {"downloads_per_30_days": 60000, "rank": 99},
                "summary": "seed",
                "version": "1.0.0",
            }
        }

        def full_scan(scan_state):
            scan_state["packages"]["openclaw"] = {
                "executable": "openclaw",
                "homepage": "https://github.com/openclaw/openclaw#readme",
                "last_updated_at": "2026-05-24T03:20:32.592Z",
                "popularity": {"downloads_per_30_days": 4247894, "rank": 0},
                "summary": "Multi-channel AI gateway",
                "version": "2026.5.22",
            }
            scan_state["last_full_scan_at"] = "2026-05-26T00:00:00+00:00"

        with (
            mock.patch.object(build_db, "_read_npm_index_state", return_value=state),
            mock.patch.object(
                build_db,
                "_current_npm_changes_sequence",
                return_value="scan-start",
            ),
            mock.patch.object(build_db, "_run_npm_full_scan", side_effect=full_scan) as scan,
            mock.patch.object(
                build_db,
                "_fetch_npm_changes_since",
                return_value=(set(), set(), "11", False),
            ) as changes,
            mock.patch.object(build_db, "_write_npm_index_state"),
        ):
            packages = build_db._collect_npm_metadata()

        scan.assert_called_once()
        changes.assert_called_once_with("scan-start")
        self.assertIn("openclaw", packages)
        self.assertLess(
            packages["openclaw"]["popularity"]["rank"],
            packages["seed-cli"]["popularity"]["rank"],
        )

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


class AvDbAuthorityTests(unittest.TestCase):
    def test_formula_metadata_adds_optional_upstream_fields_without_schema_bump(self):
        build_db = load_build_db()

        metadata = build_db._formula_metadata(
            {
                "name": "awscli",
                "desc": "AWS CLI",
                "homepage": "https://aws.amazon.com/cli/",
                "versions": {"stable": "2.34.54"},
                "urls": {
                    "stable": {
                        "url": "https://github.com/aws/aws-cli/archive/refs/tags/2.34.54.tar.gz",
                    }
                },
                "aliases": ["awscli@2"],
                "oldnames": [],
            }
        )

        self.assertEqual(build_db.SCHEMA_VERSION, 7)
        self.assertEqual(
            metadata,
            {
                "summary": "AWS CLI",
                "homepage": "https://aws.amazon.com/cli/",
                "repository": "https://github.com/aws/aws-cli",
                "version": "2.34.54",
                "sourceArchive": "https://github.com/aws/aws-cli/archive/refs/tags/2.34.54.tar.gz",
                "aliases": ["awscli@2"],
            },
        )
        self.assertNotIn("repo", metadata)

    def test_package_manager_overlay_adds_volatile_formula_fields(self):
        build_db = load_build_db()

        formulas = {"awscli": {"summary": "AWS CLI", "repository": "https://github.com/aws/aws-cli"}}
        result = build_db._overlay_formula_package_manager_metadata(
            formulas,
            [
                {
                    "name": "awscli",
                    "versions": {"stable": "2.34.54"},
                    "urls": {"stable": {"url": "https://github.com/aws/aws-cli/archive/refs/tags/2.34.54.tar.gz"}},
                }
            ],
        )

        self.assertEqual(result["awscli"]["summary"], "AWS CLI")
        self.assertEqual(result["awscli"]["version"], "2.34.54")
        self.assertEqual(result["awscli"]["sourceArchive"], "https://github.com/aws/aws-cli/archive/refs/tags/2.34.54.tar.gz")

    def test_versioned_formulae_from_package_manager_cache_enter_catalog(self):
        build_db = load_build_db()

        result = build_db._include_versioned_formula_metadata(
            {"node": {"summary": "Platform built on V8"}},
            [
                {
                    "name": "node@24",
                    "desc": "Platform built on V8 to build network applications",
                    "homepage": "https://nodejs.org/",
                    "versions": {"stable": "24.11.1"},
                    "urls": {"stable": {"url": "https://github.com/nodejs/node/archive/refs/tags/v24.11.1.tar.gz"}},
                },
                {
                    "name": "abseil",
                    "desc": "C++ library code",
                    "versions": {"stable": "20250814.1"},
                },
            ],
        )

        self.assertEqual(result["node"]["summary"], "Platform built on V8")
        self.assertIn("node@24", result)
        self.assertEqual(result["node@24"]["summary"], "Platform built on V8 to build network applications")
        self.assertEqual(result["node@24"]["homepage"], "https://nodejs.org/")
        self.assertEqual(result["node@24"]["version"], "24.11.1")
        self.assertEqual(result["node@24"]["repository"], "https://github.com/nodejs/node")
        self.assertNotIn("abseil", result)

    def test_homebrew_pulse_overlay_adds_formula_and_cask_events(self):
        build_db = load_build_db()

        def git_pulse_events(repo, keyed_paths, scope):
            if repo == build_db.HOMEWBREW_CORE_REPO:
                self.assertEqual(keyed_paths, {"Formula/a/awscli.rb": "awscli"})
                self.assertEqual(scope, "Formula")
                return {
                    "awscli": {
                        "last_updated_at": "2026-06-01T12:00:00Z",
                        "pulse_kind": "new",
                    }
                }
            if repo == build_db.HOMEWBREW_CASK_REPO:
                self.assertEqual(keyed_paths, {"Casks/1/1password-cli.rb": "1password-cli"})
                self.assertEqual(scope, "Casks")
                return {
                    "1password-cli": {
                        "last_updated_at": "2026-06-01T11:00:00Z",
                        "pulse_kind": "updated",
                    }
                }
            self.fail(f"unexpected repo {repo}")

        with mock.patch.object(build_db, "_git_pulse_events", side_effect=git_pulse_events):
            formulas, casks = build_db._overlay_homebrew_pulse_metadata(
                {"awscli": {"summary": "AWS CLI"}},
                {"1password-cli": {"summary": "1Password CLI"}},
            )

        self.assertEqual(formulas["awscli"]["last_updated_at"], "2026-06-01T12:00:00Z")
        self.assertEqual(formulas["awscli"]["pulse_kind"], "new")
        self.assertEqual(casks["1password-cli"]["last_updated_at"], "2026-06-01T11:00:00Z")
        self.assertEqual(casks["1password-cli"]["pulse_kind"], "updated")

    def test_collect_homebrew_authority_from_av_db_validates_and_returns_db_sections(self):
        build_db = load_build_db()
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "db.json")
            with open(path, "w", encoding="utf-8") as handle:
                json.dump(
                    {
                        "schema": build_db.SCHEMA_VERSION,
                        "generated_at": "2026-06-01T00:00:00+00:00",
                        "entries": {"aws": "awscli", "op": "cask:1password-cli"},
                        "formulas": {
                            "awscli": {
                                "summary": "AWS CLI",
                                "homepage": "https://aws.amazon.com/cli/",
                                "repository": "https://github.com/aws/aws-cli",
                                "docs": ["https://docs.aws.amazon.com/cli/latest/userguide"],
                                "category": "cloud-infrastructure",
                            }
                        },
                        "casks": {"1password-cli": {"binaries": [{"source": "op", "target": "op"}]}},
                        "npms": {},
                    },
                    handle,
                )

            entries, formulas, casks, missing_manifests = build_db._collect_homebrew_authority_from_av_db(path)

        self.assertEqual(entries["aws"], "awscli")
        self.assertEqual(entries["op"], "cask:1password-cli")
        self.assertEqual(formulas["awscli"]["summary"], "AWS CLI")
        self.assertEqual(formulas["awscli"]["homepage"], "https://aws.amazon.com/cli/")
        self.assertEqual(formulas["awscli"]["repository"], "https://github.com/aws/aws-cli")
        self.assertNotIn("repo", formulas["awscli"])
        self.assertEqual(formulas["awscli"]["docs"], ["https://docs.aws.amazon.com/cli/latest/userguide"])
        self.assertEqual(formulas["awscli"]["category"], "cloud-infrastructure")
        self.assertIn("1password-cli", casks)
        self.assertEqual(missing_manifests, 0)

    def test_main_uses_av_db_authority_and_overlays_npm_entries(self):
        build_db = load_build_db()
        with tempfile.TemporaryDirectory() as tmp:
            authority_path = os.path.join(tmp, "authority.json")
            formulae_path = os.path.join(tmp, "formulae.json")
            casks_path = os.path.join(tmp, "casks.json")
            output_path = os.path.join(tmp, "db.json")
            with open(authority_path, "w", encoding="utf-8") as handle:
                json.dump(
                    {
                        "schema": build_db.SCHEMA_VERSION,
                        "generated_at": "2026-06-01T00:00:00+00:00",
                        "entries": {"aws": "awscli"},
                        "formulas": {
                            "awscli": {
                                "summary": "AWS CLI",
                                "homepage": "https://aws.amazon.com/cli/",
                                "repository": "https://github.com/aws/aws-cli",
                                "docs": ["https://docs.aws.amazon.com/cli/latest/userguide"],
                                "category": "cloud-infrastructure",
                            }
                        },
                        "casks": {"1password-cli": {"binaries": [{"source": "op", "target": "op"}]}},
                        "npms": {},
                    },
                    handle,
                )
            with open(formulae_path, "w", encoding="utf-8") as handle:
                json.dump(
                    {
                        "schema": 1,
                        "formulae": [
                            {
                                "name": "awscli",
                                "versions": {"stable": "2.34.54"},
                                "urls": {"stable": {"url": "https://github.com/aws/aws-cli/archive/refs/tags/2.34.54.tar.gz"}},
                            },
                            {
                                "name": "node@24",
                                "desc": "Platform built on V8 to build network applications",
                                "homepage": "https://nodejs.org/",
                                "versions": {"stable": "24.11.1"},
                                "urls": {"stable": {"url": "https://github.com/nodejs/node/archive/refs/tags/v24.11.1.tar.gz"}},
                            }
                        ],
                    },
                    handle,
                )
            with open(casks_path, "w", encoding="utf-8") as handle:
                json.dump(
                    {
                        "schema": 1,
                        "casks": [
                            {
                                "token": "1password-cli",
                                "desc": "1Password CLI",
                                "homepage": "https://developer.1password.com/docs/cli",
                                "old_tokens": [],
                                "url": "https://example.com/op.zip",
                                "sha256": "abc123",
                                "version": "2.0.0",
                                "depends_on": {"formula": []},
                                "artifacts": [{"binary": "op"}],
                            }
                        ],
                    },
                    handle,
                )

            npm_metadata = {
                "eslint": {
                    "executable": "eslint",
                    "popularity": {"downloads_per_30_days": 100000},
                }
            }

            def git_pulse_events(repo, keyed_paths, scope):
                if repo == build_db.HOMEWBREW_CORE_REPO:
                    return {
                        "awscli": {
                            "last_updated_at": "2026-06-01T12:00:00Z",
                            "pulse_kind": "new",
                        },
                        "node@24": {
                            "last_updated_at": "2026-06-01T13:00:00Z",
                            "pulse_kind": "updated",
                        }
                    }
                if repo == build_db.HOMEWBREW_CASK_REPO:
                    return {
                        "1password-cli": {
                            "last_updated_at": "2026-06-01T11:00:00Z",
                            "pulse_kind": "updated",
                        }
                    }
                return {}

            with (
                mock.patch.object(build_db, "AUTHORITY_DB_PATH", authority_path),
                mock.patch.object(build_db, "AV_DB_FORMULAE_PATH", formulae_path),
                mock.patch.object(build_db, "AV_DB_CASKS_PATH", casks_path),
                mock.patch.object(build_db, "DB_PATH", output_path),
                mock.patch.object(build_db, "_ensure_cwd"),
                mock.patch.object(build_db, "_fetch_json", side_effect=AssertionError("legacy Homebrew fetch should not run")),
                mock.patch.object(
                    build_db,
                    "_fetch_popularity",
                    return_value={
                        "awscli": {"installs_per_365_days": 1000, "rank": 3},
                        "node@24": {"installs_per_365_days": 2400, "rank": 24},
                    },
                ),
                mock.patch.object(
                    build_db,
                    "_fetch_cask_popularity",
                    return_value={
                        "1password-cli": {"installs_per_365_days": 500, "rank": 7}
                    },
                ),
                mock.patch.object(build_db, "_git_pulse_events", side_effect=git_pulse_events),
                mock.patch.object(build_db, "_collect_npm_metadata", return_value=npm_metadata),
                mock.patch.object(build_db.sys, "argv", ["build-db.py"]),
            ):
                build_db.main()

            with open(output_path, "r", encoding="utf-8") as handle:
                db = json.load(handle)

        self.assertEqual(db["entries"]["aws"], "awscli")
        self.assertEqual(db["entries"]["eslint"], "npm:eslint")
        self.assertEqual(db["formulas"]["awscli"]["summary"], "AWS CLI")
        self.assertEqual(db["formulas"]["awscli"]["homepage"], "https://aws.amazon.com/cli/")
        self.assertEqual(db["formulas"]["awscli"]["repository"], "https://github.com/aws/aws-cli")
        self.assertNotIn("repo", db["formulas"]["awscli"])
        self.assertEqual(db["formulas"]["awscli"]["version"], "2.34.54")
        self.assertEqual(db["formulas"]["awscli"]["sourceArchive"], "https://github.com/aws/aws-cli/archive/refs/tags/2.34.54.tar.gz")
        self.assertEqual(db["formulas"]["awscli"]["docs"], ["https://docs.aws.amazon.com/cli/latest/userguide"])
        self.assertEqual(db["formulas"]["awscli"]["category"], "cloud-infrastructure")
        self.assertEqual(
            db["formulas"]["awscli"]["popularity"],
            {"installs_per_365_days": 1000, "rank": 3},
        )
        self.assertEqual(db["formulas"]["awscli"]["last_updated_at"], "2026-06-01T12:00:00Z")
        self.assertEqual(db["formulas"]["awscli"]["pulse_kind"], "new")
        self.assertEqual(db["formulas"]["node@24"]["summary"], "Platform built on V8 to build network applications")
        self.assertEqual(db["formulas"]["node@24"]["homepage"], "https://nodejs.org/")
        self.assertEqual(db["formulas"]["node@24"]["version"], "24.11.1")
        self.assertEqual(db["formulas"]["node@24"]["repository"], "https://github.com/nodejs/node")
        self.assertEqual(
            db["formulas"]["node@24"]["popularity"],
            {"installs_per_365_days": 2400, "rank": 24},
        )
        self.assertEqual(db["formulas"]["node@24"]["last_updated_at"], "2026-06-01T13:00:00Z")
        self.assertEqual(db["formulas"]["node@24"]["pulse_kind"], "updated")
        self.assertEqual(db["casks"]["1password-cli"]["url"], "https://example.com/op.zip")
        self.assertEqual(db["casks"]["1password-cli"]["sha256"], "abc123")
        self.assertEqual(db["casks"]["1password-cli"]["version"], "2.0.0")
        self.assertEqual(
            db["casks"]["1password-cli"]["popularity"],
            {"installs_per_365_days": 500, "rank": 7},
        )
        self.assertEqual(db["casks"]["1password-cli"]["last_updated_at"], "2026-06-01T11:00:00Z")
        self.assertEqual(db["casks"]["1password-cli"]["pulse_kind"], "updated")
        self.assertEqual(db["npms"], npm_metadata)

    def test_authority_db_rejects_dangling_cask_entries(self):
        build_db = load_build_db()
        with self.assertRaisesRegex(ValueError, "missing cask"):
            build_db._validate_authority_db(
                {
                    "schema": build_db.SCHEMA_VERSION,
                    "entries": {"op": "cask:1password-cli"},
                    "formulas": {"awscli": {"summary": "AWS CLI"}},
                    "casks": {"other-cli": {}},
                }
            )


if __name__ == "__main__":
    unittest.main()
