import importlib.util
import unittest
from pathlib import Path


def load_module():
    path = Path(__file__).resolve().parents[1] / "scripts" / "generate-pkg-graph.py"
    spec = importlib.util.spec_from_file_location("generate_pkg_graph", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class PackageGraphTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.module = load_module()
        cls.graph = cls.module.build_graph()

    def test_generates_agent_relationships_for_awscli(self):
        awscli = self.graph["packages"]["brew:awscli"]
        self.assertEqual(awscli["identity"]["repository"], "https://github.com/aws/aws-cli")

        related = {
            (item["rel"], item["provider"], item["name"])
            for item in awscli["linkIntents"]["relatedPackages"]
        }
        self.assertIn(("runtime_dependency", "brew", "openssl@3"), related)
        self.assertIn(("runtime_dependency", "brew", "python@3.14"), related)
        self.assertIn(("build_dependency", "brew", "cmake"), related)

        hubs = {item["slug"] for item in awscli["linkIntents"]["packageHubs"]}
        self.assertIn("cloud-clis", hubs)
        self.assertIn("secret-risk-packages", hubs)

        claims = awscli["claims"]
        self.assertTrue(any(item["intent"] == "internal-link" for item in claims))
        self.assertTrue(any(item["intent"] == "hub-backlink" for item in claims))

    def test_graph_records_relation_definitions(self):
        self.assertEqual(self.graph["schema"], 1)
        self.assertIn("runtime_dependency", self.graph["relation_definitions"])
        self.assertIn("same_software_cross_ecosystem", self.graph["relation_definitions"])
        self.assertGreater(self.graph["hubs"]["cloud-clis"]["packageCount"], 0)


if __name__ == "__main__":
    unittest.main()
