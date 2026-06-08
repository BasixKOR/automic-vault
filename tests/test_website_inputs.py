import importlib.util
import pathlib
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
WEBSITE_INPUTS_SCRIPT = ROOT / "scripts" / "export-website-inputs.py"


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class WebsiteInputsExportTests(unittest.TestCase):
    def test_website_inputs_export_product_owned_contract(self):
        module = load_module(WEBSITE_INPUTS_SCRIPT, "export_website_inputs_contract_test")
        payload = module.website_inputs()

        self.assertEqual(payload["schemaVersion"], 1)
        self.assertRegex(payload["generatedAt"], r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
        self.assertRegex(payload["productVersion"], module.PRODUCT_VERSION_RE)
        self.assertGreater(payload["scannedPackageCount"], 0)

    def test_scan_log_count_counts_numbered_rows(self):
        module = load_module(WEBSITE_INPUTS_SCRIPT, "export_website_inputs_scan_log_test")
        with tempfile.TemporaryDirectory() as tmp:
            scan_log = pathlib.Path(tmp) / "SCAN_LOG.md"
            scan_log.write_text(
                "\n".join(
                    [
                        "| # | Package |",
                        "| 1 | awscli |",
                        "| nope | ignored |",
                        "| 2 | gh |",
                        "",
                    ]
                ),
                encoding="utf-8",
            )

            self.assertEqual(module.count_scanned_packages(scan_log), 2)

    def test_product_version_reader_rejects_unexpected_versions(self):
        module = load_module(WEBSITE_INPUTS_SCRIPT, "export_website_inputs_version_test")
        with tempfile.TemporaryDirectory() as tmp:
            version_file = pathlib.Path(tmp) / "Cargo.toml"
            version_file.write_text('version = "latest"\n', encoding="utf-8")

            with self.assertRaises(SystemExit):
                module.read_product_version(version_file)


if __name__ == "__main__":
    unittest.main()
