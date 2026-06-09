import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PAGES_SCRIPT = ROOT / "scripts" / "generate-pkg-pages.py"


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class PackagePagesTests(unittest.TestCase):
    def test_agent_safety_answers_skip_missing_package_pages(self):
        module = load_module(PAGES_SCRIPT, "pkg_pages_agent_safety")
        pages = {"brew:alpha": module.PackagePage(provider="brew", name="alpha")}
        data = {
            "schema": 1,
            "priorityPackageKeys": ["brew:alpha", "brew:removed"],
            "answers": {
                "brew:alpha": {
                    "summary": "alpha runs the alpha command.",
                    "credentialAccess": "Reads local alpha credentials.",
                    "remoteMutation": "Can update remote alpha resources.",
                    "publishOrArtifactRisk": "Can publish alpha artifacts.",
                    "recommendedControl": "Gate alpha write commands.",
                    "agentUseGuidance": "Allow reads; require approval for writes.",
                },
                "brew:removed": {
                    "summary": "removed used to exist.",
                    "credentialAccess": "Could read credentials.",
                    "remoteMutation": "Could mutate remote resources.",
                    "publishOrArtifactRisk": "Could publish artifacts.",
                    "recommendedControl": "Gate writes.",
                    "agentUseGuidance": "Skip because the page is gone.",
                },
            },
        }

        module.apply_agent_safety_answers(pages, data)

        self.assertEqual(pages["brew:alpha"].agent_safety_answer["summary"], "alpha runs the alpha command.")
        self.assertIn("curated agent safety answer", pages["brew:alpha"].source_notes)


if __name__ == "__main__":
    unittest.main()
