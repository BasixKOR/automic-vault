import importlib.util
import json
import pathlib
import tempfile
import unittest
from datetime import datetime, timezone
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[1]
FRESHNESS_SCRIPT = ROOT / "scripts" / "generate-pkg-version-freshness.py"
PAGES_SCRIPT = ROOT / "scripts" / "generate-pkg-pages.py"


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PackageVersionFreshnessTests(unittest.TestCase):
    def test_version_normalization_and_comparison_are_conservative(self):
        module = load_module(FRESHNESS_SCRIPT, "pkg_version_freshness_versions")

        self.assertEqual(module.compare_versions("1.2.3", "v1.2.3"), "current")
        self.assertEqual(module.compare_versions("1.2.3", "1.2.4-beta.1"), "behind")
        self.assertEqual(module.compare_versions("1.2.4", "1.2.4-beta.1"), "current")
        self.assertEqual(module.compare_versions("12.17.0", "azure-storage-blobs_12.17.0", "azure-storage-blobs-cpp"), "unknown")
        self.assertEqual(module.compare_versions("1.2.3", "tool_1.2.4", "tool"), "behind")
        self.assertEqual(module.compare_versions("1.2.3", "release-candidate"), "unknown")

    def test_warning_thresholds_use_fixed_dates(self):
        module = load_module(FRESHNESS_SCRIPT, "pkg_version_freshness_thresholds")
        now = datetime(2026, 5, 25, tzinfo=timezone.utc)

        site = module.site_data_status(
            {
                "data/pkg-page-enrichment.json": "2026-05-18T00:00:00+00:00",
                "data/db.json": "2026-05-25T00:00:00+00:00",
            },
            now,
        )
        self.assertEqual(site["status"], "ok")

        quiet = module.manager_info(
            "brew:quiet-tool",
            {"version": "1.0.0"},
            {"formulas": {"quiet-tool": {"last_updated_at": "2025-09-01T00:00:00Z"}}},
            now,
        )
        self.assertEqual(quiet["activity"], "quiet")

        stale = module.manager_info(
            "brew:stale-tool",
            {"version": "1.0.0"},
            {"formulas": {"stale-tool": {"last_updated_at": "2025-01-01T00:00:00Z"}}},
            now,
        )
        self.assertEqual(stale["activity"], "stale")

    def test_github_release_then_tag_extraction(self):
        module = load_module(FRESHNESS_SCRIPT, "pkg_version_freshness_github")

        def fake_fetch(url, **_kwargs):
            if url.endswith("/releases/latest"):
                return {"tag_name": "v2.0.0", "html_url": "https://github.com/acme/tool/releases/tag/v2.0.0"}
            return []

        with mock.patch.object(module, "fetch_github_json", side_effect=fake_fetch):
            upstream = module.upstream_metadata(
                "brew:tool",
                {
                    "package": {"name": "tool"},
                    "version": "1.0.0",
                    "repository": "https://github.com/acme/tool",
                },
                force_refresh=False,
                cache_only=True,
            )

        self.assertEqual(upstream["latestSource"], "github_release")
        self.assertEqual(upstream["comparison"], "likely_lag")
        self.assertEqual(upstream["evidence"], "https://github.com/acme/tool/releases/tag/v2.0.0")

        def fake_tag_fetch(url, **_kwargs):
            if url.endswith("/releases/latest"):
                return None
            return [{"name": "v1.2.0"}, {"name": "v1.1.0"}]

        with mock.patch.object(module, "fetch_github_json", side_effect=fake_tag_fetch):
            upstream = module.upstream_metadata(
                "brew:tool",
                {
                    "package": {"name": "tool"},
                    "version": "1.2.0",
                    "sourceArchive": "https://github.com/acme/tool/archive/refs/tags/v1.2.0.tar.gz",
                },
                force_refresh=False,
                cache_only=True,
            )

        self.assertEqual(upstream["latestSource"], "github_tag")
        self.assertEqual(upstream["comparison"], "current")

    def test_build_freshness_records_unknown_for_noisy_or_missing_upstream(self):
        module = load_module(FRESHNESS_SCRIPT, "pkg_version_freshness_build")
        now = datetime(2026, 5, 25, tzinfo=timezone.utc)
        enrichment = {
            "generated_at": "2026-05-24T00:00:00+00:00",
            "packages": {
                "brew:local-only": {
                    "package": {"provider": "brew", "name": "local-only"},
                    "version": "1.0.0",
                },
            },
        }
        db = {
            "generated_at": "2026-05-24T00:00:00+00:00",
            "formulas": {"local-only": {"last_updated_at": "2026-05-01T00:00:00Z"}},
        }

        artifact = module.build_freshness(enrichment, db, now=now, cache_only=True)
        entry = artifact["packages"]["brew:local-only"]

        self.assertEqual(entry["siteData"]["status"], "ok")
        self.assertEqual(entry["packageManager"]["activity"], "fresh")
        self.assertEqual(entry["upstream"]["comparison"], "unknown")
        self.assertEqual(entry["warnings"][0]["kind"], "upstream_unknown")

    def test_check_fails_when_artifact_is_missing_or_stale(self):
        module = load_module(FRESHNESS_SCRIPT, "pkg_version_freshness_check")
        terminal = module.Terminal(json_mode=True)
        expected = {
            "schema": module.SCHEMA_VERSION,
            "generated_at": "2026-05-25T00:00:00+00:00",
            "input_hash": "expected",
            "packages": {"brew:one": {"packageManager": {"version": "1.0.0"}}},
        }
        with tempfile.TemporaryDirectory() as tmp:
            output = pathlib.Path(tmp) / "pkg-version-freshness.json"
            self.assertEqual(module.check_current(output, terminal), 1)
            output.write_text(
                json.dumps(
                    {
                        "schema": module.SCHEMA_VERSION,
                        "generated_at": "2026-05-25T00:00:00+00:00",
                        "input_hash": "stale",
                        "packages": {"brew:one": {"packageManager": {"version": "0.9.0"}}},
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(module, "expected_freshness", return_value=expected):
                self.assertEqual(module.check_current(output, terminal), 1)

    def test_package_pages_apply_and_render_freshness(self):
        module = load_module(PAGES_SCRIPT, "pkg_pages_freshness")
        pages = {"brew:ripgrep": module.PackagePage(provider="brew", name="ripgrep")}
        module.apply_package_version_freshness(
            pages,
            {
                "packages": {
                    "brew:ripgrep": {
                        "packageManager": {"version": "15.1.0", "updatedAt": "2026-03-20T18:56:03Z"},
                        "siteData": {"status": "ok"},
                        "upstream": {"repository": "https://github.com/BurntSushi/ripgrep", "comparison": "current", "latestVersion": "15.1.0"},
                        "warnings": [],
                    }
                }
            },
        )

        html = module.render_freshness(pages["brew:ripgrep"], {"generated_at": "2026-05-25T12:00:00+00:00"})
        self.assertIn("Version and freshness", html)
        self.assertIn("15.1.0", html)
        self.assertIn("https://github.com/BurntSushi/ripgrep", html)


if __name__ == "__main__":
    unittest.main()
