import importlib.util
import json
import pathlib
import sqlite3
import sys
import tempfile
import types
import unittest
from contextlib import closing


ROOT = pathlib.Path(__file__).resolve().parents[1]
SQLITE_SCRIPT = ROOT / "scripts" / "generate-pkg-sqlite.py"


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class FakePageModule:
    @staticmethod
    def attr(value):
        return str(value).replace('"', "&quot;")

    @staticmethod
    def locale_code(_locale):
        return "en"

    @staticmethod
    def locale_path(path, _locale):
        return path

    @staticmethod
    def tx(_locale, _key, default, **_kwargs):
        return default


class FakeBuildPageModule(FakePageModule):
    @staticmethod
    def load_sources():
        return {"fixture": True}

    @staticmethod
    def package_pages_from_sources(_sources):
        def page(name, indexable, provider="brew", slug=None):
            slug = slug or name
            return types.SimpleNamespace(
                provider=provider,
                name=name,
                slug=slug,
                path=f"/pkg/{provider}/{slug}/",
                display_name=name,
                key=f"{provider}:{name}",
                summary=f"{name} summary",
                package_manager_url=f"https://{provider}.example/{name}",
                version="1.0.0",
                category="developer-tools",
                license="MIT",
                repository="",
                homepage="",
                aliases=set(),
                executables=[],
                binaries=[],
                keywords=[],
                classifiers=[],
                package_hubs=[],
                geiger=None,
                isotope=None,
                approval_gate=None,
                extra={},
                popularity={"rank": 1},
                indexable=indexable,
                last_updated_at="2026-06-02",
            )

        return {
            "brew:ripgrep": page("ripgrep", True),
            "brew:thin": page("thin", False),
            "npm:@playwright/test": page("@playwright/test", True, "npm", "playwright-test"),
            "npm:playwright-test": page("playwright-test", True, "npm", "playwright-test"),
        }

    @staticmethod
    def package_hub_pages(_pages):
        return []

    @staticmethod
    def source_files():
        return []

    @staticmethod
    def build_manifest(_page_count, _files, _previous_manifest):
        return {"source_hash": "fixture-hash", "generated_at": "2026-06-02T19:54:51Z"}

    @staticmethod
    def i18n_locales():
        return [
            {"code": "en", "slug": "", "hreflang": "en"},
            {"code": "de", "slug": "de", "hreflang": "de"},
        ]

    @staticmethod
    def locale_code(locale):
        return "en" if locale is None else locale["code"]

    @staticmethod
    def locale_path(path, locale):
        return path if FakeBuildPageModule.locale_code(locale) == "en" else f"/de{path}"

    @staticmethod
    def render_css():
        return "body{}"

    @staticmethod
    def render_index(_pages, _hubs, _manifest, _locale):
        return '<div id="pkg-search" class="pkg-search" data-pagefind-ui></div>'

    @staticmethod
    def render_package_page(page, _manifest, _locale):
        return f"<html>{page.name}</html>"

    @staticmethod
    def render_package_markdown(page, _manifest, _locale):
        return f"# {page.name}\n"

    @staticmethod
    def render_hub_page(_hub, _hub_pages, _manifest, _locale):
        return "<html>hub</html>"

    @staticmethod
    def render_sitemap_index(_sitemap_names, _manifest):
        return "<sitemapindex><loc>https://www.automicvault.com/pkg/sitemap-brew.xml</loc></sitemapindex>"

    @staticmethod
    def render_hub_sitemap(_hubs, _manifest):
        return "<urlset></urlset>"

    @staticmethod
    def render_package_sitemap(_pages, _manifest):
        return (
            '<urlset xmlns:xhtml="http://www.w3.org/1999/xhtml">'
            '<xhtml:link rel="alternate" hreflang="de" '
            'href="https://www.automicvault.com/de/pkg/brew/ripgrep/" />'
            "</urlset>"
        )

    @staticmethod
    def is_indexable_package_page(page):
        return page.indexable

    @staticmethod
    def package_manager_label(page):
        return page.provider

    @staticmethod
    def install_command(page):
        return f"av install {page.name}"

    @staticmethod
    def native_install_command(page):
        return f"brew install {page.name}"

    @staticmethod
    def clean_summary(value):
        return value

    @staticmethod
    def short_text(value, _limit):
        return value

    @staticmethod
    def hero_sentence(page):
        return f"{page.name} hero"

    @staticmethod
    def public_copy(value):
        return value

    @staticmethod
    def taxonomy_terms(_taxonomy):
        return []

    @staticmethod
    def normalize_space(value):
        return " ".join(value.split())


class PackageSqliteTests(unittest.TestCase):
    def test_write_sqlite_records_routes_search_and_manifest(self):
        module = load_module(SQLITE_SCRIPT, "pkg_sqlite_write_test")
        with tempfile.TemporaryDirectory() as tmp:
            output = pathlib.Path(tmp) / "pkg.sqlite"
            module.write_sqlite(
                output,
                [
                    module.response_record(
                        "/pkg/",
                        "<html>packages</html>",
                        "text/html; charset=utf-8",
                        "Tue, 02 Jun 2026 19:54:51 GMT",
                    ),
                    module.response_record(
                        "/fr/pkg/",
                        "<html>paquets</html>",
                        "text/html; charset=utf-8",
                        "Tue, 02 Jun 2026 19:54:51 GMT",
                    ),
                    module.response_record(
                        "/pkg/brew/awscli/index.md",
                        "# awscli\n",
                        "text/markdown; charset=utf-8",
                        "Tue, 02 Jun 2026 19:54:51 GMT",
                    ),
                ],
                [
                    module.SearchDocument(
                        path="/pkg/brew/awscli/",
                        locale="en",
                        title="awscli",
                        summary="AWS command line interface.",
                        provider="brew",
                        package_key="brew:awscli",
                        rank=2,
                        search_text="awscli brew:awscli aws cloud cli",
                    )
                ],
                [
                    module.PackageRecord(
                        path="/pkg/brew/awscli/",
                        provider="brew",
                        slug="awscli",
                        package_key="brew:awscli",
                        name="awscli",
                        display_name="awscli",
                        summary="AWS command line interface.",
                        provider_label="Homebrew",
                        package_manager_url="https://brew.example/awscli",
                        install_command="av install awscli",
                        native_install_command="brew install awscli",
                        version="2.0.0",
                        category="developer-tools",
                        license="Apache-2.0",
                        homepage="https://aws.amazon.com/cli/",
                        repository="",
                        rank=2,
                        last_updated_at="2026-06-02",
                        indexable=True,
                        data={"aliases": ["aws"]},
                        search_text="awscli brew:awscli aws cloud cli",
                    )
                ],
                [
                    module.HubRecord(
                        path="/pkg/cloud/",
                        slug="cloud",
                        title="Cloud",
                        description="Cloud tools",
                        group="topical",
                        data={},
                    )
                ],
                [module.HubPackageRecord("cloud", "brew:awscli", 1, "Cloud CLI")],
                {
                    "schema": module.SCHEMA_VERSION,
                    "source_hash": "hash-a",
                    "manifest": {"source_hash": "hash-a"},
                },
            )

            connection = sqlite3.connect(output)
            try:
                rows = dict(connection.execute("SELECT path, content_type FROM responses"))
                metadata = {
                    key: json.loads(value)
                    for key, value in connection.execute("SELECT key, value FROM metadata")
                }
                search_rows = connection.execute(
                    "SELECT path, locale, title FROM search_documents"
                ).fetchall()
                package_rows = connection.execute(
                    "SELECT path, package_key, data_json FROM packages"
                ).fetchall()
                hub_rows = connection.execute(
                    "SELECT path, slug, title FROM hubs"
                ).fetchall()
                hub_package_rows = connection.execute(
                    "SELECT hub_slug, package_key, reason FROM hub_packages"
                ).fetchall()
                integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
            finally:
                connection.close()

        self.assertEqual(integrity, "ok")
        self.assertIn("/pkg/index.html", rows)
        self.assertIn("/fr/pkg/index.html", rows)
        self.assertEqual(rows["/pkg/brew/awscli/index.md"], "text/markdown; charset=utf-8")
        self.assertEqual(metadata["source_hash"], "hash-a")
        self.assertEqual(search_rows, [("/pkg/brew/awscli/", "en", "awscli")])
        self.assertEqual(package_rows[0][0:2], ("/pkg/brew/awscli/", "brew:awscli"))
        self.assertEqual(json.loads(package_rows[0][2])["aliases"], ["aws"])
        self.assertEqual(hub_rows, [("/pkg/cloud/", "cloud", "Cloud")])
        self.assertEqual(hub_package_rows, [("cloud", "brew:awscli", "Cloud CLI")])

    def test_source_hash_metadata_changes_between_artifacts(self):
        module = load_module(SQLITE_SCRIPT, "pkg_sqlite_hash_test")
        with tempfile.TemporaryDirectory() as tmp:
            first = pathlib.Path(tmp) / "first.sqlite"
            second = pathlib.Path(tmp) / "second.sqlite"
            for output, source_hash in ((first, "hash-a"), (second, "hash-b")):
                module.write_sqlite(
                    output,
                    [],
                    [],
                    [],
                    [],
                    [],
                    {
                        "schema": module.SCHEMA_VERSION,
                        "source_hash": source_hash,
                        "manifest": {"source_hash": source_hash},
                    },
                )
            with closing(sqlite3.connect(first)) as connection:
                first_hash = connection.execute(
                    "SELECT value FROM metadata WHERE key = 'source_hash'"
                ).fetchone()[0]
            with closing(sqlite3.connect(second)) as connection:
                second_hash = connection.execute(
                    "SELECT value FROM metadata WHERE key = 'source_hash'"
                ).fetchone()[0]

        self.assertNotEqual(first_hash, second_hash)

    def test_index_search_adapter_removes_pagefind(self):
        module = load_module(SQLITE_SCRIPT, "pkg_sqlite_adapter_test")
        html = """
<head>
  <link rel="stylesheet" href="/pagefind/pagefind-ui.css">
</head>
<body>
  <div id="pkg-search" class="pkg-search" data-pagefind-ui></div>
  <script src="/pagefind/pagefind-ui.js"></script>
  <script>
    window.addEventListener("DOMContentLoaded", () => {
      new PagefindUI({ element: "#pkg-search" });
    });
  </script>
</body>
"""

        adapted = module.adapt_package_index_search(html, FakePageModule, None)

        self.assertNotIn("pagefind", adapted)
        self.assertIn("data-av-package-search", adapted)
        self.assertIn('src="/pkg/search.js"', adapted)

    def test_build_records_includes_localized_sitemaps_and_indexable_markdown(self):
        module = load_module(SQLITE_SCRIPT, "pkg_sqlite_build_test")
        with tempfile.TemporaryDirectory() as tmp:
            responses, documents, packages, hubs, hub_packages, metadata = module.build_records(
                FakeBuildPageModule,
                pathlib.Path(tmp) / "pkg.sqlite",
            )

        response_by_path = {record.path: record for record in responses}
        package_by_key = {record.package_key: record for record in packages}
        self.assertIn("/pkg/styles.css", response_by_path)
        self.assertIn("/de/pkg/search.js", response_by_path)
        self.assertEqual(package_by_key["brew:ripgrep"].indexable, True)
        self.assertEqual(package_by_key["brew:thin"].indexable, False)
        self.assertEqual(package_by_key["brew:ripgrep"].install_command, "av install ripgrep")
        self.assertEqual(package_by_key["npm:playwright-test"].path, "/pkg/npm/playwright-test/")
        self.assertEqual(package_by_key["npm:@playwright/test"].path, "/pkg/npm/scoped-playwright-test/")
        self.assertEqual(package_by_key["npm:@playwright/test"].slug, "scoped-playwright-test")
        self.assertEqual(
            package_by_key["npm:@playwright/test"].data["full"]["path"],
            "/pkg/npm/scoped-playwright-test/",
        )
        self.assertEqual(hubs, [])
        self.assertEqual(hub_packages, [])
        self.assertEqual({document.locale for document in documents}, {"en", "de"})
        document_paths = [document.path for document in documents]
        self.assertEqual(len(document_paths), len(set(document_paths)))
        self.assertEqual(metadata["source_hash"], "fixture-hash")
        self.assertIn("last_modified", metadata)


if __name__ == "__main__":
    unittest.main()
