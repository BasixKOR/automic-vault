import importlib.util
import json
import types
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CURATION_SCRIPT = ROOT / "scripts" / "generate-pkg-graph-curation.py"
GRAPH_SCRIPT = ROOT / "scripts" / "generate-pkg-graph.py"
PAGES_SCRIPT = ROOT / "scripts" / "generate-pkg-pages.py"


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class PackageGraphCurationTests(unittest.TestCase):
    def test_validation_rejects_unknown_targets_relations_and_hubs(self):
        module = load_module(CURATION_SCRIPT, "pkg_graph_curation_validation")
        entry = {
            "linkIntents": {
                "relatedPackages": [
                    {
                        "provider": "brew",
                        "name": "missing",
                        "rel": "made_up",
                        "reason": "test",
                        "confidence": 0.8,
                        "evidence": "unit test",
                    }
                ],
                "alsoAvailableVia": [],
                "packageHubs": [{"slug": "missing-hub"}],
            }
        }

        failures = module.validate_entry(
            "brew:alpha",
            entry,
            page_keys={"brew:alpha", "brew:beta"},
            hub_slugs={"known-hub"},
        )

        joined = "\n".join(failures)
        self.assertIn("unknown relation", joined)
        self.assertIn("target does not exist locally", joined)
        self.assertIn("hub does not have a rendered definition", joined)

    def test_validation_requires_curation_for_isolated_indexable_pages(self):
        module = load_module(CURATION_SCRIPT, "pkg_graph_curation_coverage")
        fake_page = types.SimpleNamespace(
            key="brew:alpha",
            provider="brew",
            slug="alpha",
            name="alpha",
            related_packages=[],
            also_available_via=[],
            package_hubs=[],
        )
        fake_pages_module = types.SimpleNamespace(
            is_indexable_package_page=lambda _page: True,
            inferred_related_links=lambda _page: [],
        )
        artifact = {"schema": module.SCHEMA_VERSION, "packages": {}, "hubs": {}}

        failures = module.validate_curation(artifact, fake_pages_module, {"brew:alpha": fake_page})

        self.assertTrue(any("indexable isolated pages still lack curation" in failure for failure in failures))

    def test_graph_merge_preserves_deterministic_links_while_adding_curated_links(self):
        module = load_module(GRAPH_SCRIPT, "pkg_graph_curation_merge")
        graph_packages = {
            "brew:alpha": {
                "identity": {"provider": "brew", "name": "alpha"},
                "operationalContext": {},
                "linkIntents": {
                    "relatedPackages": [
                        {"provider": "brew", "name": "dep", "rel": "runtime_dependency", "reason": "dep"}
                    ],
                    "alsoAvailableVia": [],
                    "packageHubs": [],
                },
                "claims": [],
            }
        }
        curation = {
            "packages": {
                "brew:alpha": {
                    "linkIntents": {
                        "relatedPackages": [
                            {
                                "provider": "brew",
                                "name": "peer",
                                "rel": "domain_peer",
                                "reason": "same local topic",
                                "confidence": 0.64,
                                "evidence": "unit test",
                            }
                        ],
                        "alsoAvailableVia": [],
                        "packageHubs": [{"slug": "terminal-utilities", "label": "Terminal utility packages"}],
                    }
                }
            }
        }

        module.apply_curation(
            graph_packages,
            curation,
            page_keys={"brew:alpha", "brew:dep", "brew:peer"},
            enrichment_packages={},
            db={},
            geiger_packages={},
        )

        related = graph_packages["brew:alpha"]["linkIntents"]["relatedPackages"]
        self.assertEqual(related[0]["rel"], "runtime_dependency")
        self.assertEqual((related[1]["rel"], related[1]["name"]), ("domain_peer", "peer"))
        self.assertEqual(graph_packages["brew:alpha"]["linkIntents"]["packageHubs"][0]["slug"], "terminal-utilities")

    def test_current_curation_artifact_validates_against_local_pages(self):
        module = load_module(CURATION_SCRIPT, "pkg_graph_curation_current")
        artifact = json.loads((ROOT / "data" / "pkg-graph-curation.json").read_text(encoding="utf-8"))
        pages_module, pages = module.load_base_pages(artifact)

        failures = module.validate_curation(artifact, pages_module, pages)

        self.assertEqual(failures, [])

    def test_current_package_pages_have_no_isolated_indexable_pages(self):
        module = load_module(PAGES_SCRIPT, "pkg_pages_curation_coverage")
        pages = module.package_pages_from_sources(module.load_sources())
        isolated = [
            page.key
            for page in pages.values()
            if module.is_indexable_package_page(page) and not module.has_internal_package_navigation(page)
        ]

        self.assertEqual(isolated, [])


if __name__ == "__main__":
    unittest.main()
