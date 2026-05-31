import importlib.util
import json
import os
import pathlib
import tempfile
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[1]
ENRICHMENT_SCRIPT = ROOT / "scripts" / "generate-pkg-page-enrichment.py"
PAGES_SCRIPT = ROOT / "scripts" / "generate-pkg-pages.py"


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PackagePageEnrichmentTests(unittest.TestCase):
    def test_formula_enrichment_extracts_homebrew_metadata(self):
        module = load_module(ENRICHMENT_SCRIPT, "pkg_page_enrichment")
        formula = {
            "name": "awscli",
            "versions": {"stable": "2.34.50"},
            "homepage": "https://aws.amazon.com/cli/",
            "license": "Apache-2.0",
            "urls": {
                "stable": {
                    "url": "https://github.com/aws/aws-cli/archive/refs/tags/2.34.50.tar.gz",
                },
            },
            "dependencies": ["openssl@3", "python@3.14"],
            "build_dependencies": ["cmake"],
            "uses_from_macos": [{"name": "libffi"}, "mandoc"],
            "bottle": {
                "stable": {
                    "root_url": "https://ghcr.io/v2/homebrew/core",
                    "files": {
                        "arm64_sonoma": {},
                        "sonoma": {},
                    },
                },
            },
            "post_install_defined": False,
            "caveats": "Examples are installed under share/awscli/examples.",
        }
        db = {
            "entries": {
                "aws": "awscli",
                "aws_completer": "awscli",
                "ignored": "cask:aws-vault-binary",
            },
        }

        artifact = module.build_enrichment([formula], db)
        awscli = artifact["packages"]["brew:awscli"]

        self.assertEqual(awscli["version"], "2.34.50")
        self.assertEqual(awscli["homepage"], "https://aws.amazon.com/cli/")
        self.assertEqual(awscli["license"], "Apache-2.0")
        self.assertEqual(
            awscli["sourceArchive"],
            "https://github.com/aws/aws-cli/archive/refs/tags/2.34.50.tar.gz",
        )
        self.assertEqual(awscli["dependencies"], ["openssl@3", "python@3.14"])
        self.assertEqual(awscli["buildDependencies"], ["cmake"])
        self.assertEqual(awscli["usesFromMacos"], ["libffi", "mandoc"])
        self.assertEqual(awscli["bottle"]["rootUrl"], "https://ghcr.io/v2/homebrew/core")
        self.assertEqual(awscli["bottle"]["platforms"], ["arm64_sonoma", "sonoma"])
        self.assertEqual(awscli["installBehavior"]["postInstallDefined"], False)
        self.assertIn("share/awscli", awscli["installBehavior"]["caveats"])
        self.assertEqual(
            [item["name"] for item in awscli["executables"]],
            ["aws", "aws_completer"],
        )

    def test_formula_enrichment_records_service_and_skips_disabled_formulae(self):
        module = load_module(ENRICHMENT_SCRIPT, "pkg_page_enrichment_service")
        artifact = module.build_enrichment(
            [
                {
                    "name": "daemonish",
                    "versions": {"stable": "1.0.0"},
                    "service": {"run": ["daemonish"]},
                    "post_install_defined": True,
                },
                {
                    "name": "disabled-tool",
                    "disabled": True,
                    "versions": {"stable": "1.0.0"},
                },
            ],
            {"entries": {}},
        )

        self.assertNotIn("brew:disabled-tool", artifact["packages"])
        daemonish = artifact["packages"]["brew:daemonish"]
        self.assertEqual(daemonish["installBehavior"]["service"], "declared")
        self.assertEqual(daemonish["installBehavior"]["postInstallDefined"], True)
        self.assertEqual(daemonish["bottle"]["available"], False)

    def test_npm_enrichment_extracts_registry_metadata(self):
        module = load_module(ENRICHMENT_SCRIPT, "pkg_page_enrichment_npm")
        payload = {
            "dist-tags": {"latest": "3.1.5"},
            "time": {"3.1.5": "2026-03-18T19:02:50.186Z"},
            "versions": {
                "3.1.5": {
                    "name": "@11ty/eleventy",
                    "version": "3.1.5",
                    "homepage": "https://www.11ty.dev/",
                    "license": "MIT",
                    "repository": {"url": "git+https://github.com/11ty/eleventy.git"},
                    "bugs": {"url": "https://github.com/11ty/eleventy/issues"},
                    "dependencies": {"@11ty/dependency-tree": "^4.0.0", "kleur": "^4.1.5"},
                    "devDependencies": {"ava": "^6.0.0"},
                    "bin": {"eleventy": "cmd.cjs"},
                    "scripts": {"postinstall": "node scripts/postinstall.js", "prepare": "npm run build"},
                    "dist": {"tarball": "https://registry.npmjs.org/@11ty/eleventy/-/eleventy-3.1.5.tgz"},
                    "keywords": ["static-site-generator", "ssg"],
                }
            },
        }

        key, entry = module.npm_enrichment(
            "@11ty/eleventy",
            {"executable": "eleventy", "summary": "A simpler static site generator."},
            payload,
        )

        self.assertEqual(key, "npm:@11ty/eleventy")
        self.assertEqual(entry["package"]["packageManager"], "npm")
        self.assertEqual(entry["package"]["packageManagerUrl"], "https://www.npmjs.com/package/@11ty/eleventy")
        self.assertEqual(entry["version"], "3.1.5")
        self.assertEqual(entry["homepage"], "https://www.11ty.dev/")
        self.assertEqual(entry["repository"], "https://github.com/11ty/eleventy")
        self.assertEqual(entry["issueTracker"], "https://github.com/11ty/eleventy/issues")
        self.assertEqual(entry["license"], "MIT")
        self.assertEqual(entry["dependencies"], ["@11ty/dependency-tree", "kleur"])
        self.assertEqual(entry["buildDependencies"], ["ava"])
        self.assertEqual(entry["executables"][0]["name"], "eleventy")
        self.assertEqual(entry["installBehavior"]["lifecycleScripts"], ["postinstall", "prepare"])
        self.assertEqual(entry["installBehavior"]["postInstallDefined"], True)
        self.assertEqual(entry["publishedAt"], "2026-03-18T19:02:50.186Z")
        self.assertEqual(entry["sourceArchive"], "https://registry.npmjs.org/@11ty/eleventy/-/eleventy-3.1.5.tgz")

    def test_npm_enrichment_cleans_html_readme_summary(self):
        module = load_module(ENRICHMENT_SCRIPT, "pkg_page_enrichment_npm_html_summary")
        payload = {
            "dist-tags": {"latest": "0.133.0"},
            "versions": {
                "0.133.0": {
                    "name": "@openai/codex",
                    "version": "0.133.0",
                    "description": (
                        '<p align="center"><code>npm i -g @openai/codex</code><br />'
                        'or <code>brew install --cask codex</code></p> '
                        '<p align="center"><strong>Codex CLI</strong> is a coding agent '
                        'from OpenAI that runs locally on your computer. <p align="center"> '
                        '<img src="https://'
                    ),
                    "bin": {"codex": "bin/codex.js"},
                }
            },
        }

        _, entry = module.npm_enrichment("@openai/codex", {}, payload)

        self.assertEqual(
            entry["summary"],
            "Codex CLI is a coding agent from OpenAI that runs locally on your computer.",
        )
        self.assertNotIn("<", entry["summary"])
        self.assertNotIn("https://", entry["summary"])

    def test_pypi_enrichment_extracts_project_metadata(self):
        module = load_module(ENRICHMENT_SCRIPT, "pkg_page_enrichment_pypi")
        payload = {
            "info": {
                "name": "pgcli",
                "version": "4.3.0",
                "summary": "CLI for Postgres Database.",
                "home_page": "https://www.pgcli.com/",
                "license": "BSD",
                "requires_python": ">=3.9",
                "requires_dist": [
                    "click >=4.1",
                    "psycopg >=3.0.14 ; python_version >= '3.8'",
                    "setproctitle; sys_platform != 'win32'",
                ],
                "project_urls": {
                    "Source": "https://github.com/dbcli/pgcli",
                    "Documentation": "https://www.pgcli.com/",
                    "Issues": "https://github.com/dbcli/pgcli/issues",
                },
                "classifiers": [
                    "License :: OSI Approved :: BSD License",
                    "Programming Language :: Python :: 3",
                ],
            },
            "urls": [
                {
                    "packagetype": "bdist_wheel",
                    "url": "https://files.pythonhosted.org/packages/pgcli.whl",
                    "upload_time_iso_8601": "2026-04-01T12:00:00.000000Z",
                },
                {
                    "packagetype": "sdist",
                    "url": "https://files.pythonhosted.org/packages/pgcli.tar.gz",
                    "upload_time_iso_8601": "2026-04-01T12:00:00.000000Z",
                },
            ],
        }

        key, entry = module.pypi_enrichment(
            "pgcli",
            {"pythonFormula": "python@3.12", "homebrewDeps": ["libpq"]},
            payload,
        )

        self.assertEqual(key, "pip:pgcli")
        self.assertEqual(entry["package"]["packageManager"], "PyPI")
        self.assertEqual(entry["package"]["packageManagerUrl"], "https://pypi.org/project/pgcli/")
        self.assertEqual(entry["version"], "4.3.0")
        self.assertEqual(entry["summary"], "CLI for Postgres Database.")
        self.assertEqual(entry["repository"], "https://github.com/dbcli/pgcli")
        self.assertEqual(entry["upstreamDocs"], "https://www.pgcli.com/")
        self.assertEqual(entry["issueTracker"], "https://github.com/dbcli/pgcli/issues")
        self.assertEqual(entry["dependencies"], ["click", "psycopg", "setproctitle"])
        self.assertEqual(entry["executables"][0]["name"], "pgcli")
        self.assertEqual(entry["sourceArchive"], "https://files.pythonhosted.org/packages/pgcli.tar.gz")
        self.assertEqual(entry["publishedAt"], "2026-04-01T12:00:00.000000Z")
        self.assertEqual(entry["installBehavior"]["pythonRequires"], ">=3.9")
        self.assertEqual(entry["homebrewDependencies"], ["libpq"])
        self.assertEqual(entry["pythonFormula"], "python@3.12")

    def test_check_fails_when_artifact_is_missing_or_stale(self):
        module = load_module(ENRICHMENT_SCRIPT, "pkg_page_enrichment_check")
        terminal = module.Terminal(json_mode=True)
        expected = {
            "schema": module.SCHEMA_VERSION,
            "generated_at": "2026-05-23T00:00:00+00:00",
            "packages": {"brew:one": {"version": "1.0.0"}},
        }
        with tempfile.TemporaryDirectory() as tmp:
            output = pathlib.Path(tmp) / "pkg-page-enrichment.json"
            self.assertEqual(module.check_current(output, terminal), 1)
            output.write_text(
                json.dumps(
                    {
                        "schema": module.SCHEMA_VERSION,
                        "generated_at": "2026-05-22T00:00:00+00:00",
                        "packages": {"brew:one": {"version": "0.9.0"}},
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(module, "expected_enrichment", return_value=expected):
                self.assertEqual(module.check_current(output, terminal), 1)

    def test_package_page_sources_include_enrichment_artifact(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_for_enrichment_test")
        source_paths = {path.as_posix() for path in module.source_files()}
        self.assertIn("cache/pkg-page-enrichment.json", source_paths)
        self.assertIn("cache/pkg-version-freshness.json", source_paths)
        self.assertIn("data/isotopes/gh-cli/automic-vault.yml", source_paths)

    def test_package_index_reports_manifest_radioisotope_count(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_manifest_count_test")
        radioisotope_count = module.local_radioisotope_manifest_count()
        full_isotope_count = module.local_full_isotope_manifest_count()
        pages = [module.PackagePage(provider="brew", name="ripgrep")]
        manifest = {
            "source_file_count": 0,
            "radioisotope_manifest_count": radioisotope_count,
            "full_isotope_manifest_count": full_isotope_count,
            "isotope_manifest_count": radioisotope_count + full_isotope_count,
        }

        html = module.render_index(pages, [], manifest)

        self.assertEqual(radioisotope_count, len(list((ROOT / "data/radioisotopes").glob("*/automic-vault.yml"))))
        self.assertEqual(full_isotope_count, len(list((ROOT / "data/isotopes").glob("*/automic-vault.yml"))))
        self.assertIn(
            f"<div class=\"metric\"><span>radioisotopes</span><strong>{radioisotope_count:,}</strong></div>",
            html,
        )
        self.assertNotIn("<span>radioisotopes</span><strong>1</strong>", html)

    def test_tracked_radioisotope_inventory_copy_matches_manifest_counts(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_tracked_inventory_copy_test")
        radioisotope_count = module.local_radioisotope_manifest_count()
        scan_log = ROOT / "data" / "radioisotopes" / "SCAN_LOG.md"
        scanned_package_count = sum(
            1
            for line in scan_log.read_text(encoding="utf-8").splitlines()
            if line.startswith("| ") and line.split("|", 3)[1].strip().isdigit()
        )

        main_readme = (ROOT / "README.md").read_text(encoding="utf-8")
        radio_readme = (ROOT / "data/radioisotopes/README.md").read_text(encoding="utf-8")
        homepage = (ROOT / "www/index.html").read_text(encoding="utf-8")

        self.assertIn(f"- {radioisotope_count} radioisotope manifests", main_readme)
        self.assertIn(f"- Total radioisotope manifests: {radioisotope_count}", radio_readme)
        self.assertIn(f"<span>{scanned_package_count:,} Homebrew packages scanned</span>", homepage)

    def test_package_page_scope_requires_executable_surface(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_scope_policy_test")

        alias = module.PackagePage(provider="brew", name="ripgrep")
        alias.aliases.add("rg")
        self.assertTrue(module.has_executable_surface(alias))

        executable = module.PackagePage(provider="npm", name="eslint")
        executable.executables = [{"name": "eslint", "source": "test"}]
        self.assertTrue(module.has_executable_surface(executable))

        binary = module.PackagePage(provider="cask", name="iterm2")
        binary.binaries = [{"target": "iTerm"}]
        self.assertTrue(module.has_executable_surface(binary))

        security_only = module.PackagePage(provider="brew", name="vault")
        security_only.isotope = {"name": "isotope:vault"}
        self.assertFalse(module.has_executable_surface(security_only))

    def test_package_pages_from_sources_prunes_non_executable_packages(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_scope_filter_test")

        sources = {
            "db": {
                "formulas": {
                    "abseil": {"summary": "Metadata-only library."},
                    "ripgrep": {"summary": "Search tool."},
                },
                "casks": {
                    "iterm2": {
                        "summary": "Terminal app.",
                        "binaries": [{"target": "iTerm"}],
                    }
                },
                "npms": {
                    "qmd": {"summary": "No bin package."},
                },
                "entries": {"rg": "brew:ripgrep"},
            },
            "pkg_page_enrichment": {
                "packages": {
                    "npm:eslint": {
                        "package": {"provider": "npm", "name": "eslint"},
                        "executables": [{"name": "eslint", "source": "unit test"}],
                    }
                }
            },
            "geiger": {"packages": {"abseil": {"level": "yellow"}}},
        }

        pages = module.package_pages_from_sources(sources)

        self.assertIn("brew:ripgrep", pages)
        self.assertIn("cask:iterm2", pages)
        self.assertIn("npm:eslint", pages)
        self.assertNotIn("brew:abseil", pages)
        self.assertNotIn("npm:qmd", pages)

    def test_package_pages_keep_install_behavior_packages_without_executables(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_install_behavior_scope_test")

        sources = {
            "db": {
                "formulas": {
                    "openssl@3": {"summary": "Cryptography toolkit."},
                },
                "entries": {},
            },
            "pkg_page_enrichment": {
                "packages": {
                    "brew:openssl@3": {
                        "package": {"provider": "brew", "name": "openssl@3"},
                        "installBehavior": {"postInstallDefined": True},
                    }
                }
            },
        }

        pages = module.package_pages_from_sources(sources)

        self.assertIn("brew:openssl@3", pages)

    def test_thin_package_pages_are_noindex_but_security_pages_remain_indexable(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_thin_policy_test")

        thin = module.PackagePage(provider="npm", name="qmd")
        thin.executables = [{"name": "qmd", "source": "test"}]
        self.assertFalse(module.is_indexable_package_page(thin))

        enriched = module.PackagePage(provider="brew", name="ripgrep")
        enriched.summary = "Search tool"
        enriched.version = "15.0.0"
        self.assertTrue(module.is_indexable_package_page(enriched))

        security = module.PackagePage(provider="brew", name="vault")
        security.isotope = {"name": "isotope:vault"}
        self.assertTrue(module.is_indexable_package_page(security))

    def test_package_sitemap_index_points_to_ecosystem_sitemaps(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_sitemap_test")
        manifest = {"generated_at": "2026-05-24T12:00:00+00:00"}
        sitemap_index = module.render_sitemap_index(["sitemap-hubs.xml", "sitemap-brew.xml", "sitemap-npm.xml"], manifest)
        self.assertIn("<sitemapindex", sitemap_index)
        self.assertIn("https://www.automicvault.com/pkg/sitemap-brew.xml", sitemap_index)

        brew = module.PackagePage(provider="brew", name="ripgrep")
        brew.last_updated_at = "2026-05-20T00:00:00+00:00"
        package_sitemap = module.render_package_sitemap([brew], manifest)
        self.assertIn("<urlset", package_sitemap)
        self.assertIn("https://www.automicvault.com/pkg/brew/ripgrep/", package_sitemap)
        self.assertIn('xmlns:xhtml="http://www.w3.org/1999/xhtml"', package_sitemap)
        self.assertIn('hreflang="ja" href="https://www.automicvault.com/ja/pkg/brew/ripgrep/"', package_sitemap)
        self.assertIn('hreflang="x-default" href="https://www.automicvault.com/pkg/brew/ripgrep/"', package_sitemap)

    def test_package_manifest_reuses_generation_metadata_when_sources_are_unchanged(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_stable_manifest_test")

        with tempfile.TemporaryDirectory() as tmp:
            source = pathlib.Path(tmp) / "source.json"
            source.write_text('{"packages": []}\n', encoding="utf-8")
            previous = module.build_manifest(1, [source])

            newer = source.stat().st_mtime_ns + 10_000_000_000
            os.utime(source, ns=(newer, newer))
            manifest = module.build_manifest(1, [source], previous)

        self.assertEqual(manifest["source_hash"], previous["source_hash"])
        self.assertEqual(manifest["generated_at"], previous["generated_at"])
        self.assertEqual(manifest["latest_source_mtime_ns"], previous["latest_source_mtime_ns"])
        self.assertEqual(manifest["latest_source_mtime"], previous["latest_source_mtime"])

    def test_render_all_preserves_unchanged_file_metadata(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_stable_write_test")
        locale = {"code": "en", "slug": "", "htmlLang": "en", "hreflang": "en", "nativeName": "English"}
        page = module.PackagePage(provider="brew", name="ripgrep")
        page.summary = "Search tool"
        page.version = "15.0.0"
        manifest = {"schema": module.SCHEMA_VERSION, "generated_at": "2026-05-24T12:00:00+00:00", "page_count": 1}

        with tempfile.TemporaryDirectory() as tmp:
            output = pathlib.Path(tmp) / "pkg"
            with (
                mock.patch.object(module, "i18n_locales", return_value=[locale]),
                mock.patch.object(module, "non_default_i18n_locales", return_value=[]),
            ):
                module.render_all({"brew:ripgrep": page}, dict(manifest), output)
                html_path = output / "brew" / "ripgrep" / "index.html"
                old_mtime = 1_700_000_000_000_000_000
                os.utime(html_path, ns=(old_mtime, old_mtime))
                before = html_path.stat()

                stats = module.render_all({"brew:ripgrep": page}, dict(manifest), output)
                after = html_path.stat()

        self.assertEqual(stats.written, 0)
        self.assertGreater(stats.unchanged, 0)
        self.assertEqual(after.st_mtime_ns, before.st_mtime_ns)
        self.assertEqual(after.st_ctime_ns, before.st_ctime_ns)

    def test_render_all_removes_stale_generated_files(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_stale_cleanup_test")
        locale = {"code": "en", "slug": "", "htmlLang": "en", "hreflang": "en", "nativeName": "English"}
        page = module.PackagePage(provider="brew", name="ripgrep")
        page.summary = "Search tool"
        page.version = "15.0.0"
        thin_page = module.PackagePage(provider="brew", name="ripgrep")
        thin_page.executables = [{"name": "rg", "source": "unit test"}]
        manifest = {"schema": module.SCHEMA_VERSION, "generated_at": "2026-05-24T12:00:00+00:00", "page_count": 1}

        with tempfile.TemporaryDirectory() as tmp:
            output = pathlib.Path(tmp) / "pkg"
            with (
                mock.patch.object(module, "i18n_locales", return_value=[locale]),
                mock.patch.object(module, "non_default_i18n_locales", return_value=[]),
            ):
                module.render_all({"brew:ripgrep": page}, dict(manifest), output)
                markdown_path = output / "brew" / "ripgrep" / "index.md"
                self.assertTrue(markdown_path.exists())

                stats = module.render_all({"brew:ripgrep": thin_page}, dict(manifest), output)
                self.assertFalse(markdown_path.exists())

        self.assertGreaterEqual(stats.deleted, 1)

    def test_localized_package_page_uses_locale_urls_and_markdown(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_i18n_test")
        page = module.PackagePage(provider="brew", name="ripgrep")
        page.summary = "Search tool"
        page.version = "15.0.0"
        page.package_manager = "Homebrew"
        page.package_manager_url = "https://formulae.brew.sh/formula/ripgrep"
        locale = next(item for item in module.i18n_locales() if item["code"] == "ja")

        html = module.render_package_page(page, {"generated_at": "2026-05-24T12:00:00+00:00"}, locale)
        markdown = module.render_package_markdown(page, {"generated_at": "2026-05-24T12:00:00+00:00"}, locale)

        self.assertIn('<html lang="ja">', html)
        self.assertIn("<title>ripgrep をインストール | Automic Vault</title>", html)
        self.assertIn('<link rel="canonical" href="https://www.automicvault.com/ja/pkg/brew/ripgrep/">', html)
        self.assertIn('hreflang="de" href="https://www.automicvault.com/de/pkg/brew/ripgrep/"', html)
        self.assertIn("ripgrep をインストール", html)
        self.assertIn("# ripgrep をインストール", markdown)

    def test_localized_package_page_translates_site_owned_body_copy(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_fr_body_i18n_test")
        page = module.PackagePage(provider="brew", name="ripgrep")
        page.summary = "Search tool"
        page.version = "15.0.0"
        page.package_manager = "Homebrew"
        page.package_manager_url = "https://formulae.brew.sh/formula/ripgrep"
        page.homepage = "https://github.com/BurntSushi/ripgrep"
        page.repository = "https://github.com/BurntSushi/ripgrep"
        page.executables = [{"name": "rg", "kind": "binary"}]
        page.dependencies = ["pcre2"]
        page.related_packages = [{"provider": "brew", "name": "fd", "label": "fd", "reason": "Search adjacent."}]
        page.geiger = {"level": "green", "confidence": "high", "reasons": ["No service hooks."]}
        locale = next(item for item in module.i18n_locales() if item["code"] == "fr")

        html = module.render_package_page(page, {"generated_at": "2026-05-24T12:00:00+00:00"}, locale)
        markdown = module.render_package_markdown(page, {"generated_at": "2026-05-24T12:00:00+00:00"}, locale)

        self.assertIn("Résumé du paquet", html)
        self.assertIn("Commencez avec Vault", html)
        self.assertIn("Exécutables installés", html)
        self.assertIn("Version et fraîcheur", html)
        self.assertIn("Métadonnées du paquet", html)
        self.assertIn("Paquets liés", html)
        self.assertIn("Généré depuis les données du dépôt", html)
        for phrase in (
            ">Package summary<",
            ">Start with Vault",
            ">Installed executables<",
            ">Version and freshness<",
            ">Package metadata<",
            ">Related packages<",
            ">Generated from repository data<",
        ):
            self.assertNotIn(phrase, html)
        self.assertIn("## Faits du paquet", markdown)
        self.assertIn("## Notes de sécurité", markdown)
        self.assertIn("## Liens liés", markdown)

    def test_localized_package_schema_uses_locale_text(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_schema_i18n_test")
        page = module.PackagePage(provider="brew", name="ripgrep")
        page.summary = "Search tool"
        page.version = "15.0.0"
        locale = next(item for item in module.i18n_locales() if item["code"] == "fr")

        schema = module.schema_for_package(
            page,
            module.tx(locale, "metaDescription", "Install {name} with {manager}.", name=page.display_name, manager="Homebrew"),
            "2026-05-26",
            locale,
        )
        article = next(item for item in schema["@graph"] if item["@type"] == "TechArticle")
        breadcrumbs = next(item for item in schema["@graph"] if item["@type"] == "BreadcrumbList")
        how_to = next(item for item in schema["@graph"] if item["@type"] == "HowTo")

        self.assertEqual(article["headline"], "Installer ripgrep avec Homebrew")
        self.assertEqual(article["inLanguage"], "fr")
        self.assertEqual(breadcrumbs["itemListElement"][0]["name"], "Accueil")
        self.assertEqual(breadcrumbs["itemListElement"][1]["name"], "Paquets")
        self.assertEqual(how_to["name"], "Installer ripgrep")
        self.assertEqual(how_to["step"][0]["name"], "Exécuter la commande Automic Vault")

    def test_package_markdown_alternate_contains_agent_facts(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_markdown_test")
        page = module.PackagePage(provider="brew", name="ripgrep")
        page.summary = "Search tool"
        page.version = "15.0.0"
        page.package_manager = "Homebrew"
        page.package_manager_url = "https://formulae.brew.sh/formula/ripgrep"
        page.repository = "https://github.com/BurntSushi/ripgrep"
        page.dependencies = ["pcre2"]
        page.executables = [{"name": "rg", "kind": "binary"}]
        page.geiger = {"level": "green", "confidence": "high", "reasons": ["No service hooks."]}

        markdown = module.render_package_markdown(page, {"generated_at": "2026-05-24T12:00:00+00:00"})

        self.assertIn("# Install ripgrep", markdown)
        self.assertIn("brew install ripgrep", markdown)
        self.assertIn("https://formulae.brew.sh/formula/ripgrep", markdown)
        self.assertIn("- pcre2", markdown)
        self.assertIn("Geiger risk", markdown)

    def test_package_page_renderer_cleans_stale_html_summary(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_summary_cleanup_test")
        page = module.PackagePage(provider="npm", name="@openai/codex")
        page.version = "0.133.0"
        page.install_commands = [{"command": "npm install -g @openai/codex"}]
        page.summary = (
            '<p align="center"><code>npm i -g @openai/codex</code><br />'
            'or <code>brew install --cask codex</code></p> '
            "<strong>Codex CLI</strong> is a coding agent from OpenAI "
            'that runs locally on your computer. <img src="https://'
        )
        page.install_commands = module.merge_install_command_entries(
            page.install_commands,
            module.install_commands_from_summary(page, page.summary),
        )

        self.assertEqual(
            module.hero_sentence(page),
            (
                "Codex CLI is a coding agent from OpenAI that runs locally on your computer. "
                "Version 0.133.0 via npm; verified from local package data. "
                "Also installable with Homebrew Cask: brew install --cask codex."
            ),
        )
        self.assertEqual(
            module.clean_summary(page.summary),
            "Codex CLI is a coding agent from OpenAI that runs locally on your computer.",
        )
        self.assertIn("brew install --cask codex", module.render_install(page))

    def test_package_page_promotes_summary_install_command_when_local_package_exists(self):
        module = load_module(PAGES_SCRIPT, "generate_pkg_pages_verified_summary_command_test")
        sources = {
            "db": {
                "casks": {
                    "codex": {
                        "summary": "OpenAI Codex CLI.",
                        "binaries": [{"target": "codex"}],
                    }
                },
                "npms": {},
                "entries": {},
            },
            "pkg_page_enrichment": {
                "packages": {
                    "npm:@openai/codex": {
                        "package": {
                            "provider": "npm",
                            "name": "@openai/codex",
                            "packageManager": "npm",
                        },
                        "version": "0.133.0",
                        "summary": (
                            '<p><code>npm i -g @openai/codex</code><br />'
                            'or <code>brew install --cask codex</code></p>'
                            "<p><strong>Codex CLI</strong> is a coding agent from OpenAI.</p>"
                        ),
                        "executables": [{"name": "codex", "source": "bin/codex.js"}],
                    }
                }
            },
        }

        pages = module.package_pages_from_sources(sources)
        page = pages["npm:@openai/codex"]
        commands = module.install_command_entries(page)
        brew = next(item for item in commands if item["command"] == "brew install --cask codex")
        schema = module.schema_for_package(page, module.meta_description(page), "2026-05-26")
        how_to = next(item for item in schema["@graph"] if item["@type"] == "HowTo")

        self.assertEqual(brew["confidence"], 1.0)
        self.assertEqual(brew["evidence"], "local Homebrew cask metadata")
        self.assertIn("Also installable with Homebrew Cask: brew install --cask codex.", module.hero_sentence(page))
        self.assertIn("brew install --cask codex", module.meta_description(page))
        self.assertIn("../../cask/codex/", module.render_related(page))
        self.assertIn("brew install --cask codex", [step["text"] for step in how_to["step"]])


if __name__ == "__main__":
    unittest.main()
