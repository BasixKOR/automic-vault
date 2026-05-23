import importlib.util
import json
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
        self.assertIn("data/pkg-page-enrichment.json", source_paths)


if __name__ == "__main__":
    unittest.main()
