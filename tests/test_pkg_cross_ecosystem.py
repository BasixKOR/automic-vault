import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CROSS_SCRIPT = ROOT / "scripts" / "generate-pkg-cross-ecosystem.py"
GRAPH_SCRIPT = ROOT / "scripts" / "generate-pkg-graph.py"
PAGES_SCRIPT = ROOT / "scripts" / "generate-pkg-pages.py"


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class PackageCrossEcosystemTests(unittest.TestCase):
    def test_validation_rejects_missing_av_first_command(self):
        module = load_module(CROSS_SCRIPT, "pkg_cross_validation_av")
        artifact = {
            "schema": module.SCHEMA_VERSION,
            "packages": {
                "brew:alpha": {
                    "commands": [
                        {
                            "platform": "macos",
                            "manager": "Homebrew",
                            "command": "brew install alpha",
                            "kind": "package_manager",
                            "confidence": 1,
                            "evidence": "unit test",
                        }
                    ],
                    "localLinks": [],
                }
            },
        }

        failures = module.validate_artifact(artifact, {"brew:alpha"})

        self.assertTrue(any("first command must be" in failure for failure in failures))

    def test_validation_rejects_invalid_platform_and_missing_evidence(self):
        module = load_module(CROSS_SCRIPT, "pkg_cross_validation_platform")
        artifact = {
            "schema": module.SCHEMA_VERSION,
            "packages": {
                "brew:alpha": {
                    "commands": [
                        {
                            "platform": "portable",
                            "manager": "Automic Vault",
                            "command": "sudo av install brew:alpha",
                            "kind": "automic_vault",
                            "confidence": 1,
                            "evidence": "unit test",
                        },
                        {
                            "platform": "solaris",
                            "manager": "pkg",
                            "command": "pkg install alpha",
                            "kind": "package_manager",
                            "confidence": 0.5,
                        },
                    ],
                    "localLinks": [],
                }
            },
        }

        failures = module.validate_artifact(artifact, {"brew:alpha"})
        joined = "\n".join(failures)

        self.assertIn("invalid platform", joined)
        self.assertIn("missing evidence", joined)

    def test_validation_rejects_missing_local_link_target(self):
        module = load_module(CROSS_SCRIPT, "pkg_cross_validation_links")
        artifact = {
            "schema": module.SCHEMA_VERSION,
            "packages": {
                "brew:alpha": {
                    "commands": [
                        {
                            "platform": "portable",
                            "manager": "Automic Vault",
                            "command": "sudo av install brew:alpha",
                            "kind": "automic_vault",
                            "confidence": 1,
                            "evidence": "unit test",
                        }
                    ],
                    "localLinks": [
                        {
                            "provider": "npm",
                            "name": "missing",
                            "reason": "same name",
                            "confidence": 0.8,
                            "evidence": "unit test",
                        }
                    ],
                }
            },
        }

        failures = module.validate_artifact(artifact, {"brew:alpha"})

        self.assertTrue(any("target does not exist locally" in failure for failure in failures))

    def test_graph_merge_preserves_deterministic_links_while_adding_cross_links(self):
        module = load_module(GRAPH_SCRIPT, "pkg_cross_graph_merge")
        graph_packages = {
            "brew:alpha": {
                "identity": {"provider": "brew", "name": "alpha"},
                "operationalContext": {},
                "linkIntents": {
                    "relatedPackages": [],
                    "alsoAvailableVia": [
                        {
                            "provider": "npm",
                            "name": "alpha",
                            "rel": "same_software_cross_ecosystem",
                            "reason": "existing deterministic link",
                        }
                    ],
                    "packageHubs": [],
                },
                "claims": [],
            }
        }
        cross = {
            "packages": {
                "brew:alpha": {
                    "localLinks": [
                        {
                            "provider": "pip",
                            "name": "alpha",
                            "rel": "same_software_cross_ecosystem",
                            "reason": "same normalized name",
                            "confidence": 0.78,
                            "evidence": "unit test",
                        }
                    ]
                }
            }
        }

        module.apply_cross_ecosystem(
            graph_packages,
            cross,
            page_keys={"brew:alpha", "npm:alpha", "pip:alpha"},
            enrichment_packages={},
            db={},
            geiger_packages={},
        )

        also = graph_packages["brew:alpha"]["linkIntents"]["alsoAvailableVia"]
        self.assertEqual(("npm", "alpha"), (also[0]["provider"], also[0]["name"]))
        self.assertEqual(("pip", "alpha"), (also[1]["provider"], also[1]["name"]))

    def test_brew_page_renders_av_first_and_platform_commands(self):
        module = load_module(PAGES_SCRIPT, "pkg_cross_pages_brew")
        page = module.PackagePage(provider="brew", name="ripgrep")
        page.install_commands = [
            {
                "platform": "portable",
                "manager": "Automic Vault",
                "command": "sudo av install brew:ripgrep",
                "kind": "automic_vault",
                "confidence": 1,
                "evidence": "unit test",
            },
            {
                "platform": "macos",
                "manager": "Homebrew",
                "command": "brew install ripgrep",
                "kind": "package_manager",
                "confidence": 1,
                "evidence": "unit test",
            },
            {
                "platform": "linux",
                "manager": "apt",
                "command": "sudo apt install ripgrep",
                "kind": "package_manager",
                "confidence": 0.38,
                "evidence": "unit test",
            },
            {
                "platform": "windows",
                "manager": "winget",
                "command": "winget install ripgrep",
                "kind": "package_manager",
                "confidence": 0.34,
                "evidence": "unit test",
            },
        ]

        html = module.render_install(page)
        markdown = module.render_package_markdown(page, {"generated_at": "2026-05-24T12:00:00+00:00"})
        schema = module.schema_for_package(page, "Install ripgrep.", "2026-05-24")

        self.assertLess(html.index("sudo av install brew:ripgrep"), html.index("brew install ripgrep"))
        self.assertIn("macOS", html)
        self.assertIn("Linux", html)
        self.assertIn("Windows", html)
        self.assertIn("sudo av install brew:ripgrep", markdown)
        self.assertIn("brew install ripgrep", markdown)
        how_to = next(item for item in schema["@graph"] if item["@type"] == "HowTo")
        self.assertEqual(how_to["step"][0]["text"], "sudo av install brew:ripgrep")

    def test_npm_page_renders_av_first_and_brew_only_when_linked(self):
        module = load_module(PAGES_SCRIPT, "pkg_cross_pages_npm")
        page = module.PackagePage(provider="npm", name="alpha")
        page.install_commands = [
            {
                "platform": "portable",
                "manager": "Automic Vault",
                "command": "sudo av install npm:alpha",
                "kind": "automic_vault",
                "confidence": 1,
                "evidence": "unit test",
            },
            {
                "platform": "portable",
                "manager": "npm",
                "command": "npm install -g alpha",
                "kind": "package_manager",
                "confidence": 1,
                "evidence": "unit test",
            },
            {
                "platform": "macos",
                "manager": "Homebrew",
                "command": "brew install alpha",
                "kind": "package_manager",
                "confidence": 0.78,
                "evidence": "normalized local package name",
            },
        ]
        page.also_available_via = [{"provider": "brew", "name": "alpha", "reason": "same normalized name"}]

        html = module.render_install(page)
        related = module.render_related(page)

        self.assertLess(html.index("sudo av install npm:alpha"), html.index("npm install -g alpha"))
        self.assertIn("brew install alpha", html)
        self.assertIn("../../brew/alpha/", related)

    def test_current_cross_ecosystem_artifact_validates_when_present(self):
        path = ROOT / "data" / "pkg-cross-ecosystem.json"
        if not path.exists():
            self.skipTest("cross-ecosystem artifact has not been generated yet")
        module = load_module(CROSS_SCRIPT, "pkg_cross_current")
        artifact = json.loads(path.read_text(encoding="utf-8"))
        pages = module.local_pages()

        failures = module.validate_artifact(artifact, set(pages))

        self.assertEqual(failures, [])


if __name__ == "__main__":
    unittest.main()
