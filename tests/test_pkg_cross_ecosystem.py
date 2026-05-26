import importlib.util
import json
import re
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
                "confidence": 0.92,
                "evidence": "Ubuntu 24.04 LTS package indexes: ripgrep from unit test",
                "source": {
                    "type": "package_manager_index",
                    "manager": "apt",
                    "source_label": "Ubuntu 24.04 LTS package indexes",
                    "package_id": "ripgrep",
                    "package_name": "ripgrep",
                    "source_url": "unit test",
                },
            },
            {
                "platform": "windows",
                "manager": "winget",
                "command": "winget install --id BurntSushi.ripgrep -e",
                "kind": "package_manager",
                "confidence": 0.92,
                "evidence": "Windows Package Manager manifest tree: BurntSushi.ripgrep from unit test",
                "source": {
                    "type": "package_manager_index",
                    "manager": "winget",
                    "source_label": "Windows Package Manager manifest tree",
                    "package_id": "BurntSushi.ripgrep",
                    "package_name": "BurntSushi.ripgrep",
                    "source_url": "unit test",
                },
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
        self.assertIn("sudo apt install ripgrep", [step["text"] for step in how_to["step"]])

    def test_source_backed_install_rows_hide_full_source_urls(self):
        module = load_module(PAGES_SCRIPT, "pkg_cross_pages_source_display")
        item = {
            "platform": "linux",
            "manager": "dnf",
            "command": "sudo dnf install sqlite",
            "kind": "package_manager",
            "confidence": 0.92,
            "evidence": (
                "Fedora Rawhide package metadata: sqlite from "
                "https://dl.fedoraproject.org/pub/fedora/linux/development/rawhide/Everything/x86_64/os/repodata/primary.xml.zst"
            ),
            "source": {
                "type": "package_manager_index",
                "manager": "dnf",
                "source_label": "Fedora Rawhide package metadata",
                "package_id": "sqlite",
                "package_name": "sqlite",
                "source_url": "https://dl.fedoraproject.org/pub/fedora/linux/development/rawhide/Everything/x86_64/os/repodata/primary.xml.zst",
            },
        }

        html = module.install_command_row(item)

        self.assertIn("Fedora Rawhide package metadata", html)
        self.assertIn("sqlite", html)
        self.assertIn("source: dl.fedoraproject.org", html)
        self.assertIn("href=\"https://dl.fedoraproject.org/", html)
        visible_text = re.sub(r"<[^>]+>", "", html)
        self.assertNotIn("https://dl.fedoraproject.org/", visible_text)
        self.assertNotIn("/pub/fedora/linux/development/rawhide/Everything", visible_text)

    def test_howto_schema_excludes_inferred_or_low_confidence_commands(self):
        module = load_module(PAGES_SCRIPT, "pkg_cross_pages_howto_filter")
        page = module.PackagePage(provider="brew", name="alpha")
        page.install_commands = [
            {
                "platform": "portable",
                "manager": "Automic Vault",
                "command": "sudo av install brew:alpha",
                "kind": "automic_vault",
                "confidence": 1,
                "evidence": "deterministic local package key",
            },
            {
                "platform": "linux",
                "manager": "apt",
                "command": "sudo apt install alpha",
                "kind": "package_manager",
                "confidence": 0.38,
                "evidence": "agent-inferred from package-name convention",
            },
        ]

        schema = module.schema_for_package(page, "Install alpha.", "2026-05-24")
        how_to = next(item for item in schema["@graph"] if item["@type"] == "HowTo")

        self.assertEqual([step["text"] for step in how_to["step"]], ["sudo av install brew:alpha"])

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

    def test_local_curate_uses_source_backed_manager_index_matches(self):
        module = load_module(CROSS_SCRIPT, "pkg_cross_source_backed")
        manager_indexes = {
            "managers": {
                "apt": {
                    "display_name": "apt",
                    "platform": "linux",
                    "command_template": "sudo apt install {id}",
                    "source_label": "Ubuntu 24.04 LTS package indexes",
                    "packages": {
                        "nodejs": {
                            "id": "nodejs",
                            "match_names": ["node"],
                            "source_name": "nodejs",
                            "source_url": "unit test",
                        }
                    },
                }
            }
        }
        packet = {
            "target": {
                "key": "brew:node",
                "provider": "brew",
                "name": "node",
            },
            "localCandidates": [],
        }

        curated = module.local_curate_packet(packet, module.manager_matcher(manager_indexes))
        commands = curated["commands"]

        self.assertIn("sudo apt install nodejs", [item["command"] for item in commands])
        apt = next(item for item in commands if item["manager"] == "apt")
        self.assertEqual(apt["source"]["package_id"], "nodejs")
        self.assertNotIn("agent-inferred", apt["evidence"])

    def test_validation_rejects_inferred_command_evidence(self):
        module = load_module(CROSS_SCRIPT, "pkg_cross_no_inferred_validation")
        item = {
            "platform": "linux",
            "manager": "apt",
            "command": "sudo apt install alpha",
            "confidence": 0.38,
            "evidence": "agent-inferred from package-name convention",
        }

        failures = module.validate_command("brew:alpha", 1, item)

        self.assertTrue(any("inferred evidence" in failure for failure in failures))

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
