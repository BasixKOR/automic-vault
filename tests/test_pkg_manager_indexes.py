import gzip
import importlib.util
import io
import json
import sqlite3
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
INDEX_SCRIPT = ROOT / "scripts" / "generate-pkg-manager-indexes.py"


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def tar_bytes(files):
    payload = io.BytesIO()
    with tarfile.open(fileobj=payload, mode="w:gz") as tar:
        for name, text in files.items():
            data = text.encode("utf-8")
            info = tarfile.TarInfo(name)
            info.size = len(data)
            tar.addfile(info, io.BytesIO(data))
    return payload.getvalue()


class PackageManagerIndexTests(unittest.TestCase):
    def test_parse_macports_ports_json(self):
        module = load_module(INDEX_SCRIPT, "pkg_manager_macports")
        records = module.parse_macports_ports_json(
            json.dumps({"ports": [{"name": "ripgrep"}, {"name": "jq"}]}).encode(),
            "https://ports.example/ports.json",
        )

        self.assertEqual([item["id"] for item in records], ["jq", "ripgrep"])

    def test_parse_nix_packages_json_uses_attr_and_pname(self):
        module = load_module(INDEX_SCRIPT, "pkg_manager_nix")
        records = module.parse_nix_packages_json(
            json.dumps({"packages": {"legacyPackages.x86_64-linux.nodejs": {"pname": "nodejs"}}}).encode(),
            "https://nix.example/packages.json",
        )

        self.assertEqual(records[0]["id"], "legacyPackages.x86_64-linux.nodejs")
        self.assertIn("nodejs", records[0]["match_names"])

    def test_parse_debian_packages(self):
        module = load_module(INDEX_SCRIPT, "pkg_manager_apt")
        data = gzip.compress(b"Package: ripgrep\nVersion: 1\n\nPackage: nodejs\nVersion: 2\n")
        records = module.parse_debian_packages(data, "https://ubuntu.example/Packages.gz")

        self.assertEqual([item["id"] for item in records], ["nodejs", "ripgrep"])

    def test_parse_pacman_db(self):
        module = load_module(INDEX_SCRIPT, "pkg_manager_pacman")
        data = tar_bytes({"ripgrep-1/desc": "%NAME%\nripgrep\n%VERSION%\n1\n"})

        records = module.parse_pacman_db(data, "https://arch.example/extra.db.tar.gz")

        self.assertEqual(records[0]["id"], "ripgrep")

    def test_parse_apk_index(self):
        module = load_module(INDEX_SCRIPT, "pkg_manager_apk")
        data = tar_bytes({"APKINDEX": "P:ripgrep\nV:1\n\nP:nodejs\nV:2\n"})

        records = module.parse_apk_index(data, "https://alpine.example/APKINDEX.tar.gz")

        self.assertEqual([item["id"] for item in records], ["nodejs", "ripgrep"])

    def test_parse_rpm_primary(self):
        module = load_module(INDEX_SCRIPT, "pkg_manager_rpm")
        xml = b"""<?xml version="1.0"?>
<metadata xmlns="http://linux.duke.edu/metadata/common">
  <package type="rpm"><name>ripgrep</name></package>
  <package type="rpm"><name>nodejs</name></package>
</metadata>
"""

        records = module.parse_rpm_primary(gzip.compress(xml), "https://fedora.example/primary.xml.gz")

        self.assertEqual([item["id"] for item in records], ["nodejs", "ripgrep"])

    def test_parse_github_tree_for_winget_and_scoop(self):
        module = load_module(INDEX_SCRIPT, "pkg_manager_github_tree")
        winget_tree = {
            "tree": [
                {"type": "blob", "path": "manifests/B/BurntSushi/ripgrep/14.1.1/BurntSushi.ripgrep.installer.yaml"},
                {"type": "blob", "path": "manifests/B/BurntSushi/ripgrep/14.1.1/BurntSushi.ripgrep.locale.en-US.yaml"},
            ]
        }
        scoop_tree = {"tree": [{"type": "blob", "path": "bucket/ripgrep.json"}]}

        winget = module.parse_github_tree(json.dumps(winget_tree).encode(), "https://api.github.com/repos/microsoft/winget-pkgs/git/trees/master?recursive=1", "winget")
        scoop = module.parse_github_tree(json.dumps(scoop_tree).encode(), "https://api.github.com/repos/ScoopInstaller/Main/git/trees/master?recursive=1", "scoop")

        self.assertEqual(winget[0]["id"], "BurntSushi.ripgrep")
        self.assertEqual(scoop[0]["id"], "main/ripgrep")
        self.assertIn("ripgrep", scoop[0]["match_names"])

    def test_parse_winget_source_msix(self):
        module = load_module(INDEX_SCRIPT, "pkg_manager_winget_msix")
        with tempfile.NamedTemporaryFile(suffix=".db") as database:
            connection = sqlite3.connect(database.name)
            connection.executescript(
                """
                CREATE TABLE ids(rowid INTEGER PRIMARY KEY, id TEXT NOT NULL);
                CREATE TABLE names(rowid INTEGER PRIMARY KEY, name TEXT NOT NULL);
                CREATE TABLE monikers(rowid INTEGER PRIMARY KEY, moniker TEXT NOT NULL);
                CREATE TABLE manifest(rowid INTEGER PRIMARY KEY, id INT64 NOT NULL, name INT64 NOT NULL, moniker INT64 NOT NULL);
                INSERT INTO ids(rowid, id) VALUES (1, 'BurntSushi.ripgrep');
                INSERT INTO names(rowid, name) VALUES (1, 'ripgrep');
                INSERT INTO monikers(rowid, moniker) VALUES (1, 'rg');
                INSERT INTO manifest(rowid, id, name, moniker) VALUES (1, 1, 1, 1);
                """
            )
            connection.commit()
            connection.close()
            db_data = Path(database.name).read_bytes()
        payload = io.BytesIO()
        with zipfile.ZipFile(payload, mode="w") as archive:
            archive.writestr("Public/index.db", db_data)

        records = module.parse_winget_source_msix(payload.getvalue(), "https://winget.example/source.msix")

        self.assertEqual(records[0]["id"], "BurntSushi.ripgrep")
        self.assertIn("ripgrep", records[0]["match_names"])
        self.assertIn("rg", records[0]["match_names"])

    def test_parse_chocolatey_atom(self):
        module = load_module(INDEX_SCRIPT, "pkg_manager_chocolatey")
        xml = b"""<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:d="http://schemas.microsoft.com/ado/2007/08/dataservices">
  <entry><content><m:properties xmlns:m="http://schemas.microsoft.com/ado/2007/08/dataservices/metadata"><d:Id>ripgrep</d:Id></m:properties></content></entry>
  <link rel="next" href="https://chocolatey.example/next" />
</feed>
"""

        records, next_url = module.parse_chocolatey_atom(xml, "https://chocolatey.example/first")

        self.assertEqual(records[0]["id"], "ripgrep")
        self.assertEqual(next_url, "https://chocolatey.example/next")

    def test_alias_matches_add_source_backed_external_id(self):
        module = load_module(INDEX_SCRIPT, "pkg_manager_aliases")
        managers = {
            "apt": {
                "packages": {
                    "nodejs": {
                        "id": "nodejs",
                        "match_names": ["nodejs"],
                        "source_url": "https://ubuntu.example/Packages.gz",
                    }
                }
            }
        }

        module.apply_alias_matches(managers)

        self.assertIn("node", managers["apt"]["packages"]["nodejs"]["match_names"])

    def test_validate_artifact_rejects_missing_source_url(self):
        module = load_module(INDEX_SCRIPT, "pkg_manager_validation")
        artifact = {
            "schema": module.SCHEMA_VERSION,
            "definition_hash": module.stable_hash(module.MANAGER_DEFINITIONS),
            "alias_hash": module.stable_hash(module.PACKAGE_ALIAS_MATCHES),
            "managers": {
                name: {
                    "display_name": definition["display_name"],
                    "platform": definition["platform"],
                    "command_template": definition["command_template"],
                    "packages": {"ripgrep": {"id": "ripgrep", "match_names": ["ripgrep"]}},
                }
                for name, definition in module.MANAGER_DEFINITIONS.items()
            },
        }

        failures = module.validate_artifact(artifact)

        self.assertTrue(any("missing source_url" in failure for failure in failures))


if __name__ == "__main__":
    unittest.main()
