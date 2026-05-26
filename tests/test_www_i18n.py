import importlib.util
import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N_SCRIPT = ROOT / "scripts" / "generate-www-i18n.py"


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class WebsiteI18nTests(unittest.TestCase):
    def test_locale_route_generation(self):
        module = load_module(I18N_SCRIPT, "generate_www_i18n_route_test")
        ja = next(locale for locale in module.enabled_locales() if locale.code == "ja")

        self.assertEqual(module.locale_path("/", ja), "/ja/")
        self.assertEqual(module.locale_path("/docs/", ja), "/ja/docs/")
        self.assertEqual(module.href("/docs/", ja), "https://www.automicvault.com/ja/docs/")

    def test_static_hreflang_block_includes_self_and_default(self):
        module = load_module(I18N_SCRIPT, "generate_www_i18n_hreflang_test")
        locales = module.enabled_locales()

        block = module.alternate_link_block("/security/", locales)

        self.assertIn('hreflang="en" href="https://www.automicvault.com/security/"', block)
        self.assertIn('hreflang="ja" href="https://www.automicvault.com/ja/security/"', block)
        self.assertIn('hreflang="zh-Hans" href="https://www.automicvault.com/zh-hans/security/"', block)
        self.assertIn('hreflang="x-default" href="https://www.automicvault.com/security/"', block)

    def test_static_sitemap_contains_locale_alternates(self):
        module = load_module(I18N_SCRIPT, "generate_www_i18n_sitemap_test")
        locales = module.enabled_locales()
        records = [{"path": "/docs/", "dateModified": "2026-05-24"}]

        sitemap = module.render_sitemap(records, locales)

        self.assertIn('xmlns:xhtml="http://www.w3.org/1999/xhtml"', sitemap)
        self.assertIn("<loc>https://www.automicvault.com/ja/docs/</loc>", sitemap)
        self.assertIn('hreflang="fr" href="https://www.automicvault.com/fr/docs/"', sitemap)
        self.assertIn('hreflang="x-default" href="https://www.automicvault.com/docs/"', sitemap)


if __name__ == "__main__":
    unittest.main()
