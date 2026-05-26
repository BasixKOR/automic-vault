#!/usr/bin/env python3
import argparse
import copy
import datetime as dt
import hashlib
import html
import json
import os
import re
import shutil
import sys
import textwrap
import urllib.parse
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
SITE_ORIGIN = "https://www.automicvault.com"
OUTPUT_DIR = Path("www/pkg")
MANIFEST_NAME = ".manifest.json"
I18N_LOCALES_PATH = Path("data/www-i18n/locales.json")
I18N_PKG_TEMPLATES_PATH = Path("data/www-i18n/pkg/templates.json")
INDEXABLE_MIN_SIGNAL_COUNT = 2
GOOGLE_TAG = """  <!-- Google tag (gtag.js) -->
  <script async src="https://www.googletagmanager.com/gtag/js?id=G-Y78QKG1T9Y"></script>
  <script>
    window.dataLayer = window.dataLayer || [];
    function gtag(){dataLayer.push(arguments);}
    gtag('js', new Date());

    gtag('config', 'G-Y78QKG1T9Y');
  </script>"""

_I18N_LOCALES_CACHE: list[dict[str, Any]] | None = None
_I18N_PKG_TEMPLATES_CACHE: dict[str, dict[str, str]] | None = None

REQUIRED_I18N_PKG_KEYS = {
    "additionalInstallCommands",
    "alsoAvailableVia",
    "approvalGatesKicker",
    "approvalRules",
    "automicVaultInstallHeading",
    "binaries",
    "bottle",
    "buildDependencies",
    "catalogCounts",
    "catalogEyebrow",
    "catalogHubsAria",
    "catalogHubsCopy",
    "catalogHubsKicker",
    "catalogHubsTitle",
    "catalogListAria",
    "catalogPagesCopy",
    "catalogPagesKicker",
    "catalogPagesTitle",
    "catalogSearchCopy",
    "classifierConfidence",
    "classifiers",
    "command",
    "commandsAndAliases",
    "copy",
    "copyInstallCommand",
    "copyManagerInstallCommand",
    "coverageSource",
    "crawlableCatalog",
    "dependencies",
    "downloadAV",
    "executableDataMissing",
    "executables",
    "executablesTitle",
    "exposure",
    "freshness",
    "freshnessCopy",
    "freshnessTitle",
    "generatedFromRepositoryData",
    "generatedSource",
    "generatedSourceCopy",
    "geigerRisk",
    "heroPanelAria",
    "homepage",
    "homepageMissing",
    "hubCounts",
    "hubDescription",
    "hubIndexedPagesTitle",
    "hubPackageReasonAlias",
    "hubPackageReasonApproval",
    "hubPackageReasonDefault",
    "hubReviewCopy",
    "hubReviewModel",
    "hubSchemaDescription",
    "hubSummaryTitle",
    "install",
    "installBehavior",
    "installBehaviorTitle",
    "installCommand",
    "installRoutes",
    "installSourceMissing",
    "installSourcePrefix",
    "installTitleConcept",
    "installTitleConceptCopy",
    "issueTracker",
    "kind",
    "keywords",
    "license",
    "localData",
    "localReadmeExcerpt",
    "manager",
    "managerUpdated",
    "managerVersion",
    "markdownEvidence",
    "metadataEmpty",
    "metadataTitle",
    "noAliases",
    "noApprovalRules",
    "noCaveats",
    "noClassifierReasons",
    "noClassifierSignals",
    "noCrossEcosystem",
    "noFreshnessWarnings",
    "noHubMembership",
    "noPlatformNotes",
    "noRelated",
    "note",
    "overview",
    "packageCatalogTitle",
    "packageFacts",
    "packageGraph",
    "packageGraphCopy",
    "packageHubs",
    "packageKey",
    "packageManager",
    "packageManagerPage",
    "packageManagerSource",
    "packageMetadata",
    "packageMetadataKicker",
    "packageSummary",
    "pageGenerated",
    "platformInstallCommands",
    "platformNotes",
    "popularPackages",
    "published",
    "pythonFormula",
    "pythonRequires",
    "radioisotopeCoverage",
    "radioisotopeKicker",
    "radioisotopeMissingHeading",
    "radioisotopeMissingSummary",
    "recommendedReview",
    "recommendedReviewCopy",
    "related",
    "relatedLinks",
    "relatedPackages",
    "repository",
    "reviewed",
    "risk",
    "riskClassifier",
    "riskLevel",
    "schemaHowToName",
    "schemaHowToStep",
    "schemaTechArticleHeadline",
    "securityNotes",
    "securityPosture",
    "service",
    "serviceNone",
    "signals",
    "source",
    "sourceArchive",
    "sourceDatabaseAria",
    "sourceExcerpt",
    "sourceSummary",
    "sourceTrail",
    "sources",
    "sourcesCopy",
    "status",
    "summaryFallback",
    "upstream",
    "upstreamDocs",
    "upstreamLatestDetected",
    "usedSources",
    "usesFromMacos",
    "verified",
    "version",
    "versionAndFreshness",
    "why",
}


def i18n_locales() -> list[dict[str, Any]]:
    global _I18N_LOCALES_CACHE
    if _I18N_LOCALES_CACHE is not None:
        return _I18N_LOCALES_CACHE
    try:
        data = read_json(I18N_LOCALES_PATH)
    except FileNotFoundError:
        _I18N_LOCALES_CACHE = [{"code": "en", "slug": "", "htmlLang": "en", "hreflang": "en", "nativeName": "English"}]
        return _I18N_LOCALES_CACHE
    _I18N_LOCALES_CACHE = [
        item
        for item in data.get("locales", [])
        if item.get("enabled") and item.get("code")
    ]
    return _I18N_LOCALES_CACHE


def non_default_i18n_locales() -> list[dict[str, Any]]:
    return [locale for locale in i18n_locales() if locale.get("code") != "en"]


def i18n_pkg_templates() -> dict[str, dict[str, str]]:
    global _I18N_PKG_TEMPLATES_CACHE
    if _I18N_PKG_TEMPLATES_CACHE is not None:
        return _I18N_PKG_TEMPLATES_CACHE
    try:
        _I18N_PKG_TEMPLATES_CACHE = read_json(I18N_PKG_TEMPLATES_PATH, {})
    except FileNotFoundError:
        _I18N_PKG_TEMPLATES_CACHE = {}
    return _I18N_PKG_TEMPLATES_CACHE


def locale_code(locale: dict[str, Any] | None) -> str:
    return str((locale or {}).get("code") or "en")


def locale_slug(locale: dict[str, Any] | None) -> str:
    return str((locale or {}).get("slug") or "")


def locale_path(path: str, locale: dict[str, Any] | None = None) -> str:
    slug = locale_slug(locale)
    if not slug:
        return path
    if path == "/":
        return f"/{slug}/"
    return f"/{slug}{path}"


def locale_url(path: str, locale: dict[str, Any] | None = None) -> str:
    return f"{SITE_ORIGIN}{locale_path(path, locale)}"


def tx(locale: dict[str, Any] | None, key: str, default: str, **kwargs: Any) -> str:
    code = locale_code(locale)
    templates = i18n_pkg_templates().get(code, {})
    if code != "en" and key in REQUIRED_I18N_PKG_KEYS and key not in templates:
        raise KeyError(f"missing package i18n template: {code}.{key}")
    template = templates.get(key, default)
    try:
        return template.format(**kwargs)
    except (KeyError, ValueError):
        return default.format(**kwargs)


def validate_i18n_pkg_templates() -> list[str]:
    failures: list[str] = []
    templates = i18n_pkg_templates()
    for locale in non_default_i18n_locales():
        code = locale_code(locale)
        locale_templates = templates.get(code)
        if not isinstance(locale_templates, dict):
            failures.append(f"missing package i18n templates for {code}")
            continue
        missing = sorted(REQUIRED_I18N_PKG_KEYS - set(locale_templates))
        if missing:
            failures.append(f"{code} package i18n templates are missing keys: {', '.join(missing[:24])}")
    return failures


LOCALIZED_UI_ENGLISH_SNIPPETS = (
    ">Package summary<",
    ">Install with Automic Vault<",
    ">Start with Vault",
    ">Installed executables<",
    ">Version and freshness<",
    ">Package metadata<",
    ">Related packages<",
    ">Generated from repository data<",
    ">Security Notes<",
    ">Package Facts<",
)


def localized_package_ui_leaks(html_text: str) -> list[str]:
    return [snippet for snippet in LOCALIZED_UI_ENGLISH_SNIPPETS if snippet in html_text]


@dataclass
class PackagePage:
    provider: str
    name: str
    summary: str = ""
    homepage: str = ""
    version: str = ""
    last_updated_at: str = ""
    pulse_kind: str = ""
    url: str = ""
    sha256: str = ""
    binaries: list[dict[str, Any]] = field(default_factory=list)
    popularity: dict[str, Any] = field(default_factory=dict)
    aliases: set[str] = field(default_factory=set)
    source_notes: list[str] = field(default_factory=list)
    package_manager: str = ""
    package_manager_url: str = ""
    repository: str = ""
    upstream_docs: str = ""
    license: str = ""
    source_archive: str = ""
    last_verified: str = ""
    dependencies: list[str] = field(default_factory=list)
    build_dependencies: list[str] = field(default_factory=list)
    uses_from_macos: list[str] = field(default_factory=list)
    install: dict[str, Any] = field(default_factory=dict)
    install_commands: list[dict[str, Any]] = field(default_factory=list)
    executables: list[dict[str, Any]] = field(default_factory=list)
    install_behavior: dict[str, Any] = field(default_factory=dict)
    bottle: dict[str, Any] = field(default_factory=dict)
    published_at: str = ""
    keywords: list[str] = field(default_factory=list)
    issue_tracker: str = ""
    classifiers: list[str] = field(default_factory=list)
    project_urls: dict[str, str] = field(default_factory=dict)
    version_freshness: dict[str, Any] = field(default_factory=dict)
    geiger: dict[str, Any] | None = None
    related_packages: list[dict[str, Any]] = field(default_factory=list)
    also_available_via: list[dict[str, Any]] = field(default_factory=list)
    package_hubs: list[dict[str, Any]] = field(default_factory=list)
    isotope: dict[str, Any] | None = None
    isotope_readme: str = ""
    isotope_readme_html: str = ""
    isotope_readme_source: str = ""
    approval_gate: dict[str, Any] | None = None
    extra: dict[str, Any] = field(default_factory=dict)

    @property
    def key(self) -> str:
        return f"{self.provider}:{self.name}"

    @property
    def slug(self) -> str:
        return slugify(self.name)

    @property
    def path(self) -> str:
        return f"/pkg/{self.provider}/{self.slug}/"

    @property
    def display_name(self) -> str:
        if self.provider == "npm" and self.name.startswith("@"):
            return self.name
        return self.name


@dataclass(frozen=True)
class ReadmeExcerpt:
    summary: str
    html: str
    source: str


@dataclass(frozen=True)
class PackageHub:
    slug: str
    title: str
    kicker: str
    description: str
    query_terms: tuple[str, ...] = ()
    package_names: tuple[str, ...] = ()
    providers: tuple[str, ...] = ()
    risk_hub: bool = False

    @property
    def path(self) -> str:
        return f"/pkg/{self.slug}/"


PACKAGE_HUBS = (
    PackageHub(
        slug="cloud-clis",
        title="Cloud CLI packages",
        kicker="cloud command surfaces",
        description=(
            "Cloud CLIs often hold access to accounts, deploys, registries, state, "
            "and production infrastructure from a local shell."
        ),
        package_names=(
            "awscli",
            "aws-cdk",
            "azure-cli",
            "cloudflared",
            "doctl",
            "firebase-cli",
            "flyctl",
            "gcloud-cli",
            "glab",
            "google-cloud-sdk",
            "helm",
            "heroku",
            "jfrog-cli",
            "kubernetes-cli",
            "minio-mc",
            "netlify-cli",
            "oci-cli",
            "opentofu",
            "podman",
            "pulumi",
            "s3cmd",
            "s5cmd",
            "snowflake-cli",
            "terraform",
            "tfenv",
            "vercel-cli",
            "wrangler",
        ),
        query_terms=(
            "amazon web services",
            "aws",
            "azure",
            "cloudflare",
            "digitalocean",
            "docker",
            "google cloud",
            "kubernetes",
            "oci",
            "s3",
            "terraform",
        ),
    ),
    PackageHub(
        slug="source-control-tools",
        title="Source-control packages",
        kicker="repository authority",
        description=(
            "Source-control tools can read private repositories, move release tags, push commits, "
            "and publish code changes. Agent runs should not do that work without review."
        ),
        package_names=(
            "fossil",
            "gh",
            "git",
            "git-lfs",
            "glab",
            "hub",
            "jj",
            "lazygit",
            "mercurial",
            "subversion",
            "svn",
        ),
        query_terms=("source control", "version control"),
    ),
    PackageHub(
        slug="package-publishers",
        title="Package publisher tools",
        kicker="publishing authority",
        description=(
            "Package publishing tools can use registry tokens to release new artifacts, "
            "overwrite distribution metadata, and push a local agent run into the supply chain."
        ),
        package_names=(
            "cargo",
            "gem",
            "go",
            "node",
            "npm",
            "pnpm",
            "poetry",
            "python",
            "ruby",
            "rubygems",
            "twine",
            "uv",
            "yarn",
        ),
        query_terms=("package publish", "publish package", "registry token", "rubygems", "npm", "pypi", "cargo"),
    ),
    PackageHub(
        slug="mcp-tools",
        title="MCP tool packages",
        kicker="agent tool servers",
        description=(
            "Model Context Protocol tools sit between AI agents and local credentials, "
            "files, APIs, or command execution."
        ),
        query_terms=("mcp", "model context protocol"),
    ),
    PackageHub(
        slug="secret-risk-packages",
        title="Secret-risk packages",
        kicker="credential exposure",
        description=(
            "Secret-risk package pages group tools with radioisotope coverage, approval gates, or "
            "Geiger classifier findings for tools an AI agent can invoke locally."
        ),
        risk_hub=True,
    ),
)


class Terminal:
    def __init__(self, json_mode: bool = False):
        self.json_mode = json_mode
        self.use_color = (
            not json_mode
            and sys.stderr.isatty()
            and not os.environ.get("NO_COLOR")
            and os.environ.get("TERM") != "dumb"
        )
        if self.use_color:
            self.bold = "\033[1m"
            self.dim = "\033[2m"
            self.red = "\033[31m"
            self.green = "\033[32m"
            self.blue = "\033[34m"
            self.yellow = "\033[33m"
            self.reset = "\033[0m"
            self.step = "◆"
            self.ok = "✓"
            self.warn = "!"
            self.error = "✗"
        else:
            self.bold = self.dim = self.red = self.green = self.blue = self.yellow = self.reset = ""
            self.step = ">"
            self.ok = "OK"
            self.warn = "WARN"
            self.error = "ERROR"

    def log(self, message: str = "") -> None:
        if not self.json_mode:
            print(message, file=sys.stderr)

    def header(self, title: str, detail: str) -> None:
        self.log(f"{self.bold}{title}{self.reset}")
        self.log(f"{self.dim}{detail}{self.reset}")

    def step_log(self, message: str) -> None:
        self.log(f"{self.blue}{self.step}{self.reset} {self.bold}{message}{self.reset}")

    def ok_log(self, message: str) -> None:
        self.log(f"  {self.green}{self.ok}{self.reset} {message}")

    def warn_log(self, message: str) -> None:
        self.log(f"  {self.yellow}{self.warn}{self.reset} {message}")

    def error_log(self, message: str) -> None:
        self.log(f"{self.red}{self.error}{self.reset} {message}")


def ensure_cwd() -> Path:
    scripts_dir = Path(__file__).resolve().parent
    root = scripts_dir.parent
    os.chdir(root)
    return root


def read_json(path: Path, default: Any = None) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        if default is not None:
            return default
        raise


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def slugify(value: str) -> str:
    value = value.lower().strip()
    if value.startswith("@"):
        value = value[1:]
    value = value.replace("@", "-")
    value = value.replace("+", "plus")
    value = value.replace("/", "-")
    value = re.sub(r"[^a-z0-9-]+", "-", value)
    value = value.strip("-")
    return value or "package"


def normalize_space(value: Any) -> str:
    return re.sub(r"\s+", " ", str(value or "")).strip()


def short_text(value: Any, limit: int = 220) -> str:
    text = normalize_space(value)
    if len(text) <= limit:
        return text
    return text[: limit - 1].rstrip() + "…"


def paragraph_text(value: Any, limit: int = 720) -> str:
    text = normalize_space(value)
    if len(text) <= limit:
        return text
    cut = text[:limit].rsplit(" ", 1)[0]
    return cut.rstrip(".,;:") + "."


def clean_summary(value: Any) -> str:
    text = str(value or "")
    if not text:
        return ""
    text = html.unescape(text)
    text = re.sub(r"(?is)<(script|style)\b.*?</\1>", " ", text)
    text = re.sub(r"(?is)<!--.*?-->", " ", text)
    text = re.sub(r"!\[[^\]]*\]\([^)]+\)", " ", text)
    text = re.sub(r"(?i)<\s*(br|/p|/div|/li|/h[1-6])\b[^>]*>", ". ", text)
    text = re.sub(r"<[^>]+>", " ", text)
    text = re.sub(r"<[^>]*$", " ", text)
    text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    text = text.replace("`", "")
    text = re.sub(r"https?://\S*$", " ", text)
    text = normalize_space(text)
    text = re.sub(r"(\.\s*){2,}", ". ", text).strip(" ,-")
    match = re.search(
        r"\b([A-Z][A-Za-z0-9 .+/_-]{1,80}\s+is\s+[A-Za-z0-9])",
        text,
    )
    if match and re.search(r"^(npm|npx|pnpm|yarn|bun|brew|pip|uv)\s+", text, flags=re.IGNORECASE):
        text = text[match.start():]
    return paragraph_text(text, 720)


def html_escape(value: Any) -> str:
    return html.escape(str(value), quote=True)


def attr(value: Any) -> str:
    return html_escape(value)


def fmt_int(value: Any) -> str:
    if value is None or value == "":
        return ""
    try:
        return f"{int(value):,}"
    except (TypeError, ValueError):
        return str(value)


def fmt_date(value: str) -> str:
    if not value:
        return ""
    return value[:10]


def load_sources() -> dict[str, Any]:
    combined_path = Path("data/combined.json")
    if combined_path.exists():
        combined = read_json(combined_path)
        sources = combined.get("sources") or {}
        if isinstance(sources, dict):
            if Path("data/geiger-counter.json").exists():
                sources["geiger"] = read_json(Path("data/geiger-counter.json"), {})
            if Path("data/pkg-page-enrichment.json").exists():
                sources["pkg_page_enrichment"] = read_json(Path("data/pkg-page-enrichment.json"), {})
            if Path("data/pkg-version-freshness.json").exists():
                sources["pkg_version_freshness"] = read_json(Path("data/pkg-version-freshness.json"), {})
            if Path("data/pkg-graph.json").exists():
                sources["pkg_graph"] = read_json(Path("data/pkg-graph.json"), {})
            if Path("data/pkg-cross-ecosystem.json").exists():
                sources["pkg_cross_ecosystem"] = read_json(Path("data/pkg-cross-ecosystem.json"), {})
            return sources

    return {
        "aliases": read_json(Path("data/aliases.json"), {}),
        "db": read_json(Path("data/db.json"), {}),
        "geiger": read_json(Path("data/geiger-counter.json"), {}),
        "isotopes": read_json(Path("data/isotopes.json"), {}),
        "npm": read_json(Path("data/npm.json"), {}),
        "pkg_graph": read_json(Path("data/pkg-graph.json"), {}),
        "pkg_cross_ecosystem": read_json(Path("data/pkg-cross-ecosystem.json"), {}),
        "pkg_page_enrichment": read_json(Path("data/pkg-page-enrichment.json"), {}),
        "pkg_version_freshness": read_json(Path("data/pkg-version-freshness.json"), {}),
        "pip": read_json(Path("data/pip.json"), {}),
    }


def package_pages_from_sources(sources: dict[str, Any]) -> dict[str, PackagePage]:
    pages: dict[str, PackagePage] = {}
    db = sources.get("db") or {}

    def get_page(provider: str, name: str) -> PackagePage:
        key = f"{provider}:{name}"
        page = pages.get(key)
        if page is None:
            page = PackagePage(provider=provider, name=name)
            pages[key] = page
        return page

    for provider, section in (("brew", "formulas"), ("cask", "casks"), ("npm", "npms")):
        items = db.get(section) or {}
        if not isinstance(items, dict):
            continue
        for name, info in items.items():
            if not isinstance(info, dict):
                continue
            page = get_page(provider, name)
            page.summary = clean_summary(info.get("summary") or page.summary)
            page.homepage = info.get("homepage") or page.homepage
            page.version = info.get("version") or page.version
            page.last_updated_at = info.get("last_updated_at") or page.last_updated_at
            page.pulse_kind = info.get("pulse_kind") or page.pulse_kind
            page.url = info.get("url") or page.url
            page.sha256 = info.get("sha256") or page.sha256
            page.binaries = info.get("binaries") or page.binaries
            if info.get("dependencies"):
                page.dependencies = info.get("dependencies") or page.dependencies
            page.popularity = info.get("popularity") or page.popularity
            page.source_notes.append("Nucleus package database")

    for name, info in (sources.get("npm") or {}).items():
        if isinstance(info, dict):
            page = get_page("npm", name)
            page.extra.update({f"npm_{key}": value for key, value in info.items()})
            page.source_notes.append("npm overlay metadata")

    for name, info in (sources.get("pip") or {}).items():
        if isinstance(info, dict):
            page = get_page("pip", name)
            page.extra.update(info)
            page.source_notes.append("Python package overlay metadata")

    entries = db.get("entries") or {}
    if isinstance(entries, dict):
        for executable, provider_key in entries.items():
            if not isinstance(provider_key, str):
                continue
            if ":" in provider_key:
                provider, name = provider_key.split(":", 1)
                if provider == "formula":
                    provider = "brew"
            else:
                provider, name = "brew", provider_key
            if provider in {"brew", "cask", "npm", "pip"}:
                get_page(provider, name).aliases.add(executable)

    stub_exclusions = sources.get("stub_exclusions") or {}
    if isinstance(stub_exclusions, dict):
        for package_key, excluded in stub_exclusions.items():
            if not isinstance(package_key, str) or ":" not in package_key:
                continue
            provider, name = package_key.split(":", 1)
            if provider in {"brew", "cask", "npm", "pip"} and isinstance(excluded, list):
                page = get_page(provider, name)
                page.extra["stub_exclusions"] = sorted(str(item) for item in excluded)

    geiger_packages = (sources.get("geiger") or {}).get("packages") or {}
    if isinstance(geiger_packages, dict):
        for name, geiger in geiger_packages.items():
            if isinstance(geiger, dict):
                page = get_page("brew", name)
                page.geiger = geiger
                page.source_notes.append("Geiger risk classifier")

    for alias, provider_key in (sources.get("aliases") or {}).items():
        if not isinstance(provider_key, str) or ":" not in provider_key:
            continue
        provider, name = provider_key.split(":", 1)
        if provider in {"brew", "cask", "npm", "pip"}:
            get_page(provider, name).aliases.add(alias)

    isotope_by_package = isotope_metadata_by_package(sources.get("isotopes") or {})
    for package_key, isotope in isotope_by_package.items():
        provider, name = package_key.split(":", 1)
        page = get_page(provider, name)
        page.isotope = isotope
        page.source_notes.append("radioisotope security manifest")

    readmes = radioisotope_readmes()
    fork_readmes = isotope_fork_readmes()
    for page in pages.values():
        if page.isotope:
            isotope_name = str(page.isotope.get("name") or "").removeprefix("isotope:")
            readme = readmes.get(isotope_name)
            if not readme:
                repository_name = str(page.isotope.get("repository") or "").rsplit("/", 1)[-1]
                directory_name = str(page.isotope.get("directory") or "")
                readme = fork_readmes.get(directory_name) or fork_readmes.get(repository_name)
            if readme:
                page.isotope_readme = readme.summary
                page.isotope_readme_html = readme.html
                page.isotope_readme_source = readme.source
                page.source_notes.append("local isotope README")

    for package_key, gate in approval_gate_metadata_by_package().items():
        provider, name = package_key.split(":", 1)
        page = get_page(provider, name)
        page.approval_gate = gate
        page.source_notes.append("approval-gate seed metadata")

    apply_package_page_enrichment(pages, sources.get("pkg_page_enrichment") or {})
    apply_package_version_freshness(pages, sources.get("pkg_version_freshness") or {})
    apply_package_page_supplements(pages)
    pages = executable_package_pages(pages)
    apply_package_graph(pages, sources.get("pkg_graph") or {})
    apply_package_cross_ecosystem(pages, sources.get("pkg_cross_ecosystem") or {})
    verify_local_install_commands(pages)

    return pages


def apply_package_page_enrichment(pages: dict[str, PackagePage], enrichment: dict[str, Any]) -> None:
    packages = enrichment.get("packages") if isinstance(enrichment, dict) else None
    if not isinstance(packages, dict):
        return
    for package_key, info in packages.items():
        if not isinstance(package_key, str) or ":" not in package_key or not isinstance(info, dict):
            continue
        provider, name = package_key.split(":", 1)
        if provider not in {"brew", "cask", "npm", "pip"} or not name:
            continue
        page = pages.setdefault(package_key, PackagePage(provider=provider, name=name))
        package = info.get("package") or {}
        if isinstance(package, dict):
            page.package_manager = package.get("packageManager") or page.package_manager
            page.package_manager_url = package.get("packageManagerUrl") or page.package_manager_url
        raw_summary = info.get("summary") or page.summary
        page.install_commands = merge_install_command_entries(
            page.install_commands,
            install_commands_from_summary(page, raw_summary),
        )
        page.summary = clean_summary(raw_summary)
        page.homepage = info.get("homepage") or page.homepage
        page.repository = info.get("repository") or page.repository
        page.upstream_docs = info.get("upstreamDocs") or page.upstream_docs
        page.version = info.get("version") or page.version
        page.license = info.get("license") or page.license
        page.source_archive = info.get("sourceArchive") or page.source_archive
        page.dependencies = info.get("dependencies") or page.dependencies
        page.build_dependencies = info.get("buildDependencies") or page.build_dependencies
        page.uses_from_macos = info.get("usesFromMacos") or page.uses_from_macos
        page.executables = info.get("executables") or page.executables
        page.install_behavior = info.get("installBehavior") or page.install_behavior
        page.bottle = info.get("bottle") or page.bottle
        page.published_at = info.get("publishedAt") or page.published_at
        page.keywords = info.get("keywords") or page.keywords
        page.issue_tracker = info.get("issueTracker") or page.issue_tracker
        page.classifiers = info.get("classifiers") or page.classifiers
        page.project_urls = info.get("projectUrls") or page.project_urls
        page.extra["homebrewDeps"] = info.get("homebrewDependencies") or page.extra.get("homebrewDeps")
        page.extra["pythonFormula"] = info.get("pythonFormula") or page.extra.get("pythonFormula")
        page.source_notes.append("package-page enrichment")


def apply_package_version_freshness(pages: dict[str, PackagePage], freshness: dict[str, Any]) -> None:
    packages = freshness.get("packages") if isinstance(freshness, dict) else None
    if not isinstance(packages, dict):
        return
    for package_key, info in packages.items():
        if not isinstance(package_key, str) or ":" not in package_key or not isinstance(info, dict):
            continue
        provider, name = package_key.split(":", 1)
        if provider not in {"brew", "cask", "npm", "pip"} or not name:
            continue
        page = pages.setdefault(package_key, PackagePage(provider=provider, name=name))
        page.version_freshness = info
        page.source_notes.append("package version freshness")


def apply_package_page_supplements(pages: dict[str, PackagePage]) -> None:
    base = Path("data/pkg-pages")
    if not base.exists():
        return
    for path in sorted(base.glob("*/*.json")):
        supplement = read_json(path, {})
        package = supplement.get("package") or {}
        provider = package.get("provider") or path.parent.name
        name = package.get("name") or path.stem
        if provider not in {"brew", "cask", "npm", "pip"} or not name:
            continue
        page = pages.setdefault(f"{provider}:{name}", PackagePage(provider=provider, name=name))
        raw_summary = supplement.get("summary") or page.summary
        page.install_commands = merge_install_command_entries(
            page.install_commands,
            install_commands_from_summary(page, raw_summary),
        )
        page.summary = clean_summary(raw_summary)
        page.homepage = supplement.get("homepage") or page.homepage
        page.version = supplement.get("version") or page.version
        page.last_verified = supplement.get("lastVerified") or page.last_verified
        page.package_manager = package.get("packageManager") or page.package_manager
        page.package_manager_url = package.get("packageManagerUrl") or page.package_manager_url
        page.repository = supplement.get("repository") or page.repository
        page.upstream_docs = supplement.get("upstreamDocs") or page.upstream_docs
        page.license = supplement.get("license") or page.license
        page.source_archive = supplement.get("sourceArchive") or page.source_archive
        page.dependencies = supplement.get("dependencies") or page.dependencies
        page.build_dependencies = supplement.get("buildDependencies") or page.build_dependencies
        page.uses_from_macos = supplement.get("usesFromMacos") or page.uses_from_macos
        page.install = supplement.get("install") or page.install
        page.executables = supplement.get("executables") or page.executables
        page.install_behavior = supplement.get("installBehavior") or page.install_behavior
        page.bottle = supplement.get("bottle") or page.bottle
        page.published_at = supplement.get("publishedAt") or page.published_at
        page.keywords = supplement.get("keywords") or page.keywords
        page.issue_tracker = supplement.get("issueTracker") or page.issue_tracker
        page.related_packages = supplement.get("relatedPackages") or page.related_packages
        page.also_available_via = supplement.get("alsoAvailableVia") or page.also_available_via
        page.source_notes.append(f"package-page supplement {path.as_posix()}")


def apply_package_graph(pages: dict[str, PackagePage], graph: dict[str, Any]) -> None:
    packages = graph.get("packages") if isinstance(graph, dict) else None
    if not isinstance(packages, dict):
        return
    for package_key, graph_entry in packages.items():
        if not isinstance(package_key, str) or ":" not in package_key or not isinstance(graph_entry, dict):
            continue
        provider, name = package_key.split(":", 1)
        if provider not in {"brew", "cask", "npm", "pip"} or not name:
            continue
        page = pages.get(package_key)
        if page is None:
            continue
        identity = graph_entry.get("identity") or {}
        if isinstance(identity, dict):
            page.repository = identity.get("repository") or page.repository
            if not page.upstream_docs and identity.get("homepage") and identity.get("homepage") != page.repository:
                page.upstream_docs = identity.get("homepage") or page.upstream_docs
        intents = graph_entry.get("linkIntents") or {}
        if not isinstance(intents, dict):
            continue
        page.related_packages = merge_related_links(
            page.related_packages,
            intents.get("relatedPackages") if isinstance(intents.get("relatedPackages"), list) else [],
        )
        page.also_available_via = merge_related_links(
            page.also_available_via,
            intents.get("alsoAvailableVia") if isinstance(intents.get("alsoAvailableVia"), list) else [],
        )
        page.package_hubs = merge_hub_links(
            page.package_hubs,
            intents.get("packageHubs") if isinstance(intents.get("packageHubs"), list) else [],
        )
        page.source_notes.append("package relationship graph")


def apply_package_cross_ecosystem(pages: dict[str, PackagePage], cross_ecosystem: dict[str, Any]) -> None:
    packages = cross_ecosystem.get("packages") if isinstance(cross_ecosystem, dict) else None
    if not isinstance(packages, dict):
        return
    for package_key, entry in packages.items():
        if not isinstance(package_key, str) or not isinstance(entry, dict):
            continue
        page = pages.get(package_key)
        if page is None:
            continue
        commands = entry.get("commands")
        if isinstance(commands, list):
            page.install_commands = merge_install_command_entries(
                [item for item in commands if isinstance(item, dict)],
                page.install_commands,
            )
        page.also_available_via = merge_related_links(
            page.also_available_via,
            entry.get("localLinks") if isinstance(entry.get("localLinks"), list) else [],
            limit=12,
        )
        page.source_notes.append("cross-ecosystem install command graph")


def merge_related_links(existing: list[dict[str, Any]], generated: list[Any], limit: int = 24) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    seen: set[tuple[str, str]] = set()
    for item in list(existing) + [item for item in generated if isinstance(item, dict)]:
        provider = str(item.get("provider") or "").strip()
        name = str(item.get("name") or "").strip()
        if not provider or not name:
            continue
        key = (provider, name)
        if key in seen:
            continue
        seen.add(key)
        result.append(item)
        if len(result) >= limit:
            break
    return result


def merge_hub_links(existing: list[dict[str, Any]], generated: list[Any]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    seen: set[str] = set()
    for item in list(existing) + [item for item in generated if isinstance(item, dict)]:
        slug = str(item.get("slug") or "").strip()
        if not slug or slug in seen:
            continue
        seen.add(slug)
        result.append(item)
    return result


def native_command_package_key(command_text: str) -> tuple[str, str] | None:
    text = normalize_space(command_text)
    patterns = (
        (r"^brew\s+install\s+--cask\s+([A-Za-z0-9@._/+~-]+)$", "cask"),
        (r"^brew\s+install\s+(?!--cask\b)([A-Za-z0-9@._/+~-]+)$", "brew"),
        (r"^(?:npm\s+(?:install|i)\s+-g|npm\s+-g\s+(?:install|i))\s+(@?[A-Za-z0-9._/+~-]+)$", "npm"),
        (r"^pip\s+install\s+([A-Za-z0-9._/+~-]+)$", "pip"),
    )
    for pattern, provider in patterns:
        match = re.match(pattern, text)
        if match:
            return provider, match.group(1)
    return None


def verified_install_evidence(provider: str) -> str:
    return {
        "brew": "local Homebrew formula metadata",
        "cask": "local Homebrew cask metadata",
        "npm": "local npm package metadata",
        "pip": "local PyPI package metadata",
    }.get(provider, "local package metadata")


def verify_local_install_commands(pages: dict[str, PackagePage]) -> None:
    for page in pages.values():
        verified_commands: list[dict[str, Any]] = []
        verified_links: list[dict[str, Any]] = []
        for item in page.install_commands:
            if not isinstance(item, dict):
                continue
            command_key = native_command_package_key(str(item.get("command") or ""))
            if command_key is None:
                verified_commands.append(item)
                continue
            provider, name = command_key
            target_key = f"{provider}:{name}"
            target = pages.get(target_key)
            if target is None or target.key == page.key:
                verified_commands.append(item)
                continue
            verified_commands.append({
                **item,
                "confidence": 1.0,
                "evidence": verified_install_evidence(provider),
            })
            verified_links.append({
                "provider": provider,
                "name": name,
                "label": target.display_name,
                "rel": "same_software_cross_ecosystem",
                "reason": "Install command points at a matching local package page.",
                "confidence": 1.0,
                "evidence": verified_install_evidence(provider),
            })
        page.install_commands = merge_install_command_entries(verified_commands)
        if verified_links:
            page.also_available_via = merge_related_links(page.also_available_via, verified_links, limit=12)
            page.source_notes.append("local install command verification")


def merge_install_command_entries(*groups: list[dict[str, Any]]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    seen: set[str] = set()
    for group in groups:
        for item in group:
            if not isinstance(item, dict):
                continue
            command_text = normalize_space(item.get("command") or "")
            if not command_text or command_text in seen:
                continue
            seen.add(command_text)
            result.append({**item, "command": command_text})
    return result


def install_commands_from_summary(page: PackagePage, value: Any) -> list[dict[str, Any]]:
    text = html.unescape(str(value or ""))
    if not text:
        return []
    text = re.sub(r"<[^>]+>", " ", text)
    text = normalize_space(text)
    commands: list[dict[str, Any]] = []

    def add(platform: str, manager: str, command_text: str, provider: str) -> None:
        if provider == page.provider:
            return
        commands.append({
            "platform": platform,
            "manager": manager,
            "command": command_text,
            "kind": "package_manager",
            "confidence": 0.72,
            "evidence": "package summary install note",
        })

    for match in re.finditer(r"(?<!\S)brew\s+install\s+--cask\s+([A-Za-z0-9@._/+~-]+)", text):
        add("macos", "Homebrew Cask", f"brew install --cask {match.group(1)}", "cask")
    for match in re.finditer(r"(?<!\S)brew\s+install\s+(?!--cask\b)([A-Za-z0-9@._/+~-]+)", text):
        add("macos", "Homebrew", f"brew install {match.group(1)}", "brew")
    for match in re.finditer(r"(?<!\S)(?:npm\s+(?:install|i)\s+-g|npm\s+-g\s+(?:install|i))\s+(@?[A-Za-z0-9._/+~-]+)", text):
        add("portable", "npm", f"npm install -g {match.group(1)}", "npm")
    for match in re.finditer(r"(?<!\S)pip\s+install\s+([A-Za-z0-9._/+~-]+)", text):
        add("portable", "pip", f"pip install {match.group(1)}", "pip")
    return merge_install_command_entries(commands)


def isotope_metadata_by_package(isotopes: dict[str, Any]) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for isotope_name, isotope in isotopes.items():
        if not isinstance(isotope, dict):
            continue
        modifies = isotope.get("modifies") or isotope.get("replaces")
        if not isinstance(modifies, str) or ":" not in modifies:
            continue
        provider, name = modifies.split(":", 1)
        if provider == "formula":
            provider = "brew"
        if provider not in {"brew", "cask", "npm", "pip"}:
            continue
        enriched = dict(isotope)
        enriched.setdefault("directory", isotope_name)
        result[f"{provider}:{name}"] = enriched
    return result


def radioisotope_readmes() -> dict[str, ReadmeExcerpt]:
    readmes: dict[str, ReadmeExcerpt] = {}
    base = Path("data/radioisotopes")
    if not base.exists():
        return readmes
    for path in base.iterdir():
        if not path.is_dir() or path.name.startswith("."):
            continue
        readme = path / "README.md"
        if readme.exists():
            text = readme.read_text(encoding="utf-8", errors="replace")
            readmes[path.name] = ReadmeExcerpt(
                summary=summarize_markdown(text),
                html=render_markdown_excerpt(text),
                source=readme.as_posix(),
            )
    return readmes


def isotope_fork_readmes() -> dict[str, ReadmeExcerpt]:
    readmes: dict[str, ReadmeExcerpt] = {}
    base = Path("data/isotopes")
    if not base.exists():
        return readmes
    for readme in sorted(base.glob("*/README.md")):
        text = trim_isotope_fork_readme(readme.read_text(encoding="utf-8", errors="replace"))
        if not text.strip():
            continue
        readmes[readme.parent.name] = ReadmeExcerpt(
            summary=summarize_markdown(text),
            html=render_markdown_excerpt(text),
            source=readme.as_posix(),
        )
    return readmes


def trim_isotope_fork_readme(text: str) -> str:
    lines: list[str] = []
    for line in text.splitlines():
        if re.search(r"\b(remainder|rest) of this README\b.*\boriginal upstream\b", line, flags=re.IGNORECASE):
            break
        lines.append(line)
    while lines and not lines[-1].strip():
        lines.pop()
    while lines and re.fullmatch(r"-{3,}", lines[-1].strip()):
        lines.pop()
    return "\n".join(lines).strip() + "\n" if lines else ""


def approval_gate_metadata_by_package() -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for path in sorted(Path("data/approval-gates").glob("*/*.yaml")):
        text = path.read_text(encoding="utf-8", errors="replace")
        namespace = match_yaml_scalar(text, r"package:\s*(?:\n|\r\n)(?:.*\n)*?\s+namespace:\s*([^\n#]+)")
        name = match_yaml_scalar(text, r"package:\s*(?:\n|\r\n)(?:.*\n)*?\s+name:\s*([^\n#]+)")
        if namespace == "formula":
            namespace = "brew"
        if not namespace or not name:
            namespace = path.parent.name
            name = path.stem
        if namespace == "brew":
            provider = "brew"
        elif namespace in {"cask", "npm", "pip"}:
            provider = namespace
        else:
            continue
        rules = parse_approval_rules(text)
        descriptions = [rule.get("description", "") for rule in rules if rule.get("description")]
        severities = [rule.get("severity", "") for rule in rules if rule.get("severity")]
        entrypoints = re.findall(r"^\s+-\s+name:\s*([^\n#]+)", text, flags=re.MULTILINE)
        analytics_rank = match_yaml_scalar(text, r"^\s+rank:\s*([^\n#]+)")
        reviewed_at = match_yaml_scalar(text, r"^\s+reviewedAt:\s*([^\n#]+)")
        coverage_status = match_yaml_scalar(text, r"^\s+status:\s*([^\n#]+)")
        result[f"{provider}:{name}"] = {
            "path": str(path),
            "rule_count": len(rules),
            "rules": [clean_yaml_scalar(item) for item in descriptions[:7]],
            "severities": sorted({clean_yaml_scalar(item) for item in severities}),
            "entrypoints": sorted({clean_yaml_scalar(item) for item in entrypoints[:8]}),
            "analytics_rank": clean_yaml_scalar(analytics_rank),
            "reviewed_at": clean_yaml_scalar(reviewed_at),
            "coverage_status": clean_yaml_scalar(coverage_status),
        }
    return result


def parse_approval_rules(text: str) -> list[dict[str, str]]:
    rules: list[dict[str, str]] = []
    in_rules = False
    current: dict[str, str] | None = None
    for line in text.splitlines():
        if re.match(r"^rules:\s*$", line):
            in_rules = True
            continue
        if not in_rules:
            continue
        if line and not line.startswith(" "):
            break
        rule_id = re.match(r"^\s+-\s+id:\s*([^\n#]+)", line)
        if rule_id:
            current = {"id": clean_yaml_scalar(rule_id.group(1))}
            rules.append(current)
            continue
        if current is None:
            continue
        description = re.match(r"^\s+description:\s*(.+)$", line)
        if description:
            current["description"] = clean_yaml_scalar(description.group(1))
            continue
        severity = re.match(r"^\s+severity:\s*([^\n#]+)", line)
        if severity:
            current["severity"] = clean_yaml_scalar(severity.group(1))
            continue
    return rules


def match_yaml_scalar(text: str, pattern: str) -> str:
    match = re.search(pattern, text, flags=re.MULTILINE)
    if not match:
        return ""
    return clean_yaml_scalar(match.group(1))


def clean_yaml_scalar(value: Any) -> str:
    text = str(value or "").strip()
    if text.startswith(('"', "'")) and text.endswith(('"', "'")):
        text = text[1:-1]
    return text.strip()


def summarize_markdown(text: str) -> str:
    text = re.sub(r"```.*?```", " ", text, flags=re.DOTALL)
    lines: list[str] = []
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or stripped.startswith("|"):
            continue
        stripped = re.sub(r"\[([^\]]+)\]\([^\)]+\)", r"\1", stripped)
        stripped = stripped.replace("`", "")
        lines.append(stripped)
    return paragraph_text(" ".join(lines), 720)


def render_markdown_excerpt(text: str) -> str:
    html_blocks: list[str] = []
    paragraph: list[str] = []
    list_type = ""
    code_lines: list[str] | None = None

    def close_paragraph() -> None:
        if paragraph:
            html_blocks.append(f"<p>{render_inline_markdown(' '.join(paragraph))}</p>")
            paragraph.clear()

    def close_list() -> None:
        nonlocal list_type
        if list_type:
            html_blocks.append(f"</{list_type}>")
            list_type = ""

    def open_list(kind: str) -> None:
        nonlocal list_type
        close_paragraph()
        if list_type != kind:
            close_list()
            html_blocks.append(f"<{kind}>")
            list_type = kind

    for raw_line in text.replace("\r\n", "\n").split("\n"):
        line = raw_line.rstrip()
        stripped = line.strip()
        if code_lines is not None:
            if stripped.startswith("```"):
                html_blocks.append(f"<pre><code>{html_escape(chr(10).join(code_lines))}</code></pre>")
                code_lines = None
            else:
                code_lines.append(line)
            continue
        if stripped.startswith("```"):
            close_paragraph()
            close_list()
            code_lines = []
            continue
        if not stripped:
            close_paragraph()
            close_list()
            continue
        if re.match(r"^\[[^\]]+\]:\s+", stripped):
            continue
        heading = re.match(r"^(#{1,6})\s+(.+)$", stripped)
        if heading:
            close_paragraph()
            close_list()
            tag = "h3" if len(heading.group(1)) == 1 else "h4"
            html_blocks.append(f"<{tag}>{render_inline_markdown(heading.group(2))}</{tag}>")
            continue
        unordered = re.match(r"^[-*]\s+(.+)$", stripped)
        if unordered:
            open_list("ul")
            html_blocks.append(f"<li>{render_inline_markdown(unordered.group(1))}</li>")
            continue
        ordered = re.match(r"^\d+\.\s+(.+)$", stripped)
        if ordered:
            open_list("ol")
            html_blocks.append(f"<li>{render_inline_markdown(ordered.group(1))}</li>")
            continue
        close_list()
        paragraph.append(stripped)

    if code_lines is not None:
        html_blocks.append(f"<pre><code>{html_escape(chr(10).join(code_lines))}</code></pre>")
    close_paragraph()
    close_list()
    return "\n".join(html_blocks)


def render_inline_markdown(text: str) -> str:
    text = re.sub(r"!\[([^\]]*)\]\([^)]+\)", r"\1", text)
    pieces: list[str] = []
    index = 0
    while index < len(text):
        if text[index] == "`":
            end = text.find("`", index + 1)
            if end != -1:
                pieces.append(f"<code>{html_escape(text[index + 1:end])}</code>")
                index = end + 1
                continue
        if text[index] == "[":
            close = text.find("]", index + 1)
            if close != -1 and close + 1 < len(text) and text[close + 1] == "(":
                url_end = text.find(")", close + 2)
                if url_end != -1:
                    label = text[index + 1:close]
                    url = text[close + 2:url_end].strip()
                    if is_public_url(url):
                        pieces.append(f'<a href="{attr(url)}">{render_inline_markdown(label)}</a>')
                    else:
                        pieces.append(html_escape(label))
                    index = url_end + 1
                    continue
        pieces.append(html_escape(text[index]))
        index += 1
    return "".join(pieces)


def is_public_url(url: str) -> bool:
    return url.startswith("https://") or url.startswith("http://")


def source_files() -> list[Path]:
    files: list[Path] = []
    data = Path("data")
    for path in data.iterdir() if data.exists() else []:
        if path.is_file() and path.suffix in {".json", ".jsonc", ".md"}:
            files.append(path)
    for root in (Path("data/radioisotopes"), Path("data/approval-gates")):
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            parts = set(path.parts)
            if ".git" in parts or path.name == ".DS_Store":
                continue
            files.append(path)
    supplement_root = Path("data/pkg-pages")
    if supplement_root.exists():
        files.extend(path for path in supplement_root.rglob("*.json") if path.is_file())
    i18n_root = Path("data/www-i18n")
    if i18n_root.exists():
        files.extend(path for path in i18n_root.rglob("*.json") if path.is_file())
    isotope_root = Path("data/isotopes")
    if isotope_root.exists():
        files.extend(path for path in isotope_root.glob("*/README.md") if path.is_file())
    return sorted(set(files))


def source_digest(files: list[Path]) -> tuple[str, int]:
    digest = hashlib.sha256()
    latest = 0
    for path in files:
        stat = path.stat()
        latest = max(latest, stat.st_mtime_ns)
        rel = path.as_posix()
        digest.update(rel.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest(), latest


def build_manifest(page_count: int, files: list[Path]) -> dict[str, Any]:
    digest, latest = source_digest(files)
    latest_dt = dt.datetime.fromtimestamp(latest / 1_000_000_000, dt.timezone.utc)
    return {
        "schema": SCHEMA_VERSION,
        "generated_at": utc_now(),
        "source_hash": digest,
        "source_file_count": len(files),
        "latest_source_mtime_ns": latest,
        "latest_source_mtime": latest_dt.replace(microsecond=0).isoformat(),
        "page_count": page_count,
    }


def package_index_signals(page: PackagePage) -> list[str]:
    signals: list[str] = []
    for attr_name in (
        "summary",
        "homepage",
        "version",
        "license",
        "package_manager_url",
        "repository",
        "upstream_docs",
        "source_archive",
        "issue_tracker",
        "published_at",
    ):
        if getattr(page, attr_name):
            signals.append(attr_name)
    for attr_name in (
        "dependencies",
        "build_dependencies",
        "uses_from_macos",
        "keywords",
        "classifiers",
    ):
        if getattr(page, attr_name):
            signals.append(attr_name)
    if page.executables or page.aliases or page.binaries:
        signals.append("commands")
    if any(value for value in page.install_behavior.values()):
        signals.append("install_behavior")
    if page.bottle and (page.bottle.get("available") or page.bottle.get("platforms")):
        signals.append("bottle")
    if page.isotope:
        signals.append("radioisotope")
    if page.approval_gate:
        signals.append("approval_gate")
    if page.geiger:
        signals.append("geiger")
    if page.related_packages or page.also_available_via or page.package_hubs:
        signals.append("relationships")
    if any(page.extra.get(key) for key in ("homebrewDeps", "pythonFormula", "stub_exclusions")):
        signals.append("local_overlay")
    return signals


def is_indexable_package_page(page: PackagePage) -> bool:
    if page.isotope or page.approval_gate or page.geiger:
        return True
    return len(package_index_signals(page)) >= INDEXABLE_MIN_SIGNAL_COUNT


def has_executable_surface(page: PackagePage) -> bool:
    return bool(page.aliases or page.executables or page.binaries)


def has_package_page_surface(page: PackagePage) -> bool:
    return has_executable_surface(page) or any(value for value in page.install_behavior.values())


def executable_package_pages(pages: dict[str, PackagePage]) -> dict[str, PackagePage]:
    return {
        key: page
        for key, page in pages.items()
        if has_package_page_surface(page)
    }


def package_hub_pages(pages: list[PackagePage]) -> list[tuple[PackageHub, list[PackagePage]]]:
    hubs: list[tuple[PackageHub, list[PackagePage]]] = []
    static_slugs = {hub.slug for hub in PACKAGE_HUBS}
    for hub in PACKAGE_HUBS:
        matches = sorted(
            [page for page in pages if package_matches_hub(page, hub)],
            key=hub_sort_key,
        )
        if matches:
            hubs.append((hub, matches))
    dynamic_hubs: dict[str, dict[str, str]] = {}
    for page in pages:
        for item in page.package_hubs:
            if not isinstance(item, dict):
                continue
            slug = str(item.get("slug") or "").strip()
            if not slug or slug in static_slugs:
                continue
            dynamic_hubs.setdefault(slug, {
                "title": str(item.get("label") or slug.replace("-", " ").title()),
                "kicker": str(item.get("kicker") or "package graph cluster"),
                "description": str(item.get("description") or item.get("reason") or "Package graph hub generated from local package facts."),
            })
    for slug, info in sorted(dynamic_hubs.items(), key=lambda item: item[1]["title"].lower()):
        hub = PackageHub(
            slug=slug,
            title=info["title"],
            kicker=info["kicker"],
            description=info["description"],
        )
        matches = sorted(
            [
                page for page in pages
                if any(isinstance(item, dict) and item.get("slug") == slug for item in page.package_hubs)
            ],
            key=hub_sort_key,
        )
        if matches:
            hubs.append((hub, matches))
    return hubs


def package_matches_hub(page: PackagePage, hub: PackageHub) -> bool:
    if hub.providers and page.provider not in hub.providers:
        return False
    if hub.risk_hub:
        if page.isotope or page.approval_gate:
            return True
        level = str((page.geiger or {}).get("level") or "").lower()
        return level not in {"", "green", "low", "unknown"}
    names = {page.name.lower(), page.slug.lower(), page.display_name.lower()}
    names.update(alias.lower() for alias in page.aliases)
    names.update(str(item.get("name") or "").lower() for item in page.executables if isinstance(item, dict))
    if any(name in names for name in hub.package_names):
        return True
    haystack = " ".join(
        str(value or "")
        for value in (
            page.name,
            clean_summary(page.summary),
            " ".join(page.aliases),
        )
    ).lower()
    return any(hub_term_matches(haystack, term) for term in hub.query_terms)


def hub_term_matches(haystack: str, term: str) -> bool:
    escaped = re.escape(term.lower())
    if re.search(r"\s", term):
        return re.search(rf"(?<![a-z0-9]){escaped}(?![a-z0-9])", haystack) is not None
    return re.search(rf"(?<![a-z0-9]){escaped}(?![a-z0-9])", haystack) is not None


def hub_sort_key(page: PackagePage) -> tuple[int, int, int, str, str]:
    risk_rank = {"critical": 0, "high": 1, "medium": 2, "yellow": 3, "low": 4, "green": 5}
    level = str((page.geiger or {}).get("level") or "").lower()
    coverage = 0 if page.isotope else 1
    gated = 0 if page.approval_gate else 1
    rank = int(page.popularity.get("rank") or 999999)
    return (coverage, gated, risk_rank.get(level, 6), rank, page.display_name.lower())


def render_all(pages: dict[str, PackagePage], manifest: dict[str, Any], output_dir: Path) -> None:
    locales = i18n_locales()
    generated_dirs = [output_dir] + [output_dir.parent / locale_slug(locale) / "pkg" for locale in non_default_i18n_locales()]
    for generated_dir in generated_dirs:
        if generated_dir.exists():
            shutil.rmtree(generated_dir)
        generated_dir.mkdir(parents=True, exist_ok=True)
    ordered = sorted(pages.values(), key=lambda page: (page.provider, page.slug, page.name))
    hubs = package_hub_pages(ordered)
    indexable_pages = [page for page in ordered if is_indexable_package_page(page)]
    sitemap_names = ["sitemap-hubs.xml"] + [
        f"sitemap-{provider}.xml"
        for provider in ("brew", "cask", "npm", "pip")
        if any(page.provider == provider for page in indexable_pages)
    ]
    manifest["hub_count"] = len(hubs)
    manifest["indexable_page_count"] = len(indexable_pages)
    manifest["noindex_page_count"] = len(ordered) - len(indexable_pages)
    manifest["markdown_page_count"] = len(indexable_pages)
    manifest["sitemap_count"] = len(sitemap_names)
    manifest["sitemap_page_counts"] = {
        provider: sum(1 for page in indexable_pages if page.provider == provider)
        for provider in ("brew", "cask", "npm", "pip")
        if any(page.provider == provider for page in indexable_pages)
    }
    for locale in locales:
        localized_dir = output_dir if locale_code(locale) == "en" else output_dir.parent / locale_slug(locale) / "pkg"
        (localized_dir / "styles.css").write_text(render_css(), encoding="utf-8")
        for page in ordered:
            page_dir = localized_dir / page.provider / page.slug
            page_dir.mkdir(parents=True, exist_ok=True)
            (page_dir / "index.html").write_text(render_package_page(page, manifest, locale), encoding="utf-8")
            if is_indexable_package_page(page):
                (page_dir / "index.md").write_text(render_package_markdown(page, manifest, locale), encoding="utf-8")
        for hub, hub_pages in hubs:
            hub_dir = localized_dir / hub.slug
            hub_dir.mkdir(parents=True, exist_ok=True)
            (hub_dir / "index.html").write_text(render_hub_page(hub, hub_pages, manifest, locale), encoding="utf-8")
        (localized_dir / "index.html").write_text(render_index(ordered, hubs, manifest, locale), encoding="utf-8")
        if locale_code(locale) == "en":
            (localized_dir / "sitemap.xml").write_text(render_sitemap_index(sitemap_names, manifest), encoding="utf-8")
            (localized_dir / "sitemap-hubs.xml").write_text(render_hub_sitemap(hubs, manifest), encoding="utf-8")
            for provider in ("brew", "cask", "npm", "pip"):
                provider_pages = [page for page in indexable_pages if page.provider == provider]
                if provider_pages:
                    (localized_dir / f"sitemap-{provider}.xml").write_text(render_package_sitemap(provider_pages, manifest), encoding="utf-8")
        (localized_dir / MANIFEST_NAME).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def render_index(
    pages: list[PackagePage],
    hubs: list[tuple[PackageHub, list[PackagePage]]],
    manifest: dict[str, Any],
    locale: dict[str, Any] | None = None,
) -> str:
    secured = [page for page in pages if page.isotope]
    gated = [page for page in pages if page.approval_gate]
    top_pages = sorted(
        pages,
        key=lambda page: int(page.popularity.get("rank") or 999999),
    )[:72]
    package_links = "\n".join(
        f'<a class="package-row" href="{locale_path(page.path, locale)}"><span>{html_escape(page.display_name)}</span><small>{html_escape(label_for(page, locale))}</small></a>'
        for page in top_pages
    )
    hub_links = "\n".join(
        f'<a class="hub-card" href="{locale_path(hub.path, locale)}"><span>{html_escape(hub.title)}</span><strong>{fmt_int(len(hub_pages))}</strong><small>{html_escape(hub.kicker)}</small></a>'
        for hub, hub_pages in hubs
    )
    search_placeholder = json.dumps(tx(locale, "searchPlaceholder", "Search awscli, gh, .env, npm publish"), ensure_ascii=False)
    return html_doc(
        title=tx(locale, "packageCatalogTitle", "Package security catalog") + " | Automic Vault",
        description=tx(locale, "packageCatalogDescription", (
            "Automic Vault package catalog for executable Nucleus packages, radioisotope "
            "secret handling, approval gates, install metadata, and agent security notes."
        )),
        canonical=locale_url("/pkg/", locale),
        alternates_path="/pkg/",
        locale=locale,
        body=f"""
{nav('../', locale)}
<main>
  <section class="pkg-hero pkg-hero-index" aria-labelledby="pkg-title">
    <div class="hero-copy">
      <p class="eyebrow">{html_escape(tx(locale, 'catalogEyebrow', 'Nucleus package intelligence'))}</p>
      <h1 id="pkg-title">{html_escape(tx(locale, "packageCatalogTitle", "Package security catalog"))}</h1>
      <p class="lede">{html_escape(tx(locale, 'catalogPagesCopy', 'Generated pages for executable packages Nucleus knows about, with local radioisotope manifests, approval-gate metadata, install popularity, executable aliases, and upstream package facts.'))}</p>
    </div>
    <aside class="hero-panel" aria-label="{attr(tx(locale, 'catalogCounts', 'Catalog counts'))}">
      {metric(tx(locale, 'packages', 'packages'), fmt_int(len(pages)))}
      {metric(tx(locale, 'radioisotopes', 'radioisotopes'), fmt_int(len(secured)))}
      {metric(tx(locale, 'approvalGates', 'approval gates'), fmt_int(len(gated)))}
      {metric(tx(locale, 'sourceFiles', 'source files'), fmt_int(manifest.get('source_file_count')))}
    </aside>
  </section>
  <section class="pkg-section pkg-search-section" aria-labelledby="pkg-search-title">
    <div class="search-copy">
      <p class="section-kicker">{html_escape(tx(locale, 'siteSearch', 'site search'))}</p>
      <h2 id="pkg-search-title">{html_escape(tx(locale, 'findPackageCoverage', 'Find package coverage'))}</h2>
      <p>{html_escape(tx(locale, 'catalogSearchCopy', 'Search generated package pages, security guides, documentation, and source-backed metadata from one index.'))}</p>
    </div>
    <div id="pkg-search" class="pkg-search" data-pagefind-ui></div>
  </section>
  <section class="pkg-section" aria-labelledby="pkg-hubs-title">
    <p class="section-kicker">{html_escape(tx(locale, 'catalogHubsKicker', 'package hubs'))}</p>
    <h2 id="pkg-hubs-title">{html_escape(tx(locale, 'catalogHubsTitle', 'Package groups with security signals'))}</h2>
    <p>{html_escape(tx(locale, 'catalogHubsCopy', 'These crawlable hubs group package families that matter for agent security: cloud CLIs, source-control tools, package publishers, MCP tools, and packages with local secret-risk signals.'))}</p>
    <div class="hub-grid" aria-label="{attr(tx(locale, 'catalogHubsAria', 'Package category hubs'))}">
      {hub_links}
    </div>
  </section>
  <section class="pkg-section split-section">
    <div>
      <p class="section-kicker">{html_escape(tx(locale, 'catalogPagesKicker', 'crawlable catalog'))}</p>
      <h2>{html_escape(tx(locale, 'catalogPagesTitle', 'Package pages from local source data'))}</h2>
      <p>{html_escape(tx(locale, 'crawlableCatalog', 'Nucleus package metadata, generated package inventories, radioisotope READMEs, secret migration manifests, and approval-gate seeds are written to static HTML so search and answer engines can find specific tool coverage.'))}</p>
    </div>
    <div class="package-list" aria-label="{attr(tx(locale, 'popularPackages', 'Popular packages'))}">
      {package_links}
    </div>
  </section>
</main>
{footer('../', locale)}
""",
        stylesheet_href=locale_path("/pkg/styles.css", locale),
        favicon_href="/favicon.ico",
        extra_head='  <link rel="stylesheet" href="/pagefind/pagefind-ui.css">',
        extra_body=f'''  <script src="/pagefind/pagefind-ui.js"></script>
  <script>
    window.addEventListener("DOMContentLoaded", () => {{
      new PagefindUI({{
        element: "#pkg-search",
        showImages: false,
        showSubResults: true,
        pageSize: 8,
        excerptLength: 24,
        resetStyles: false,
        translations: {{
          placeholder: {search_placeholder}
        }}
      }});
    }});
  </script>''',
        schema={
            "@context": "https://schema.org",
            "@type": "CollectionPage",
            "name": tx(locale, "packageCatalogTitle", "Automic Vault package security catalog"),
            "url": locale_url("/pkg/", locale),
            "inLanguage": (locale or {}).get("htmlLang") or "en",
            "isPartOf": {"@type": "WebSite", "name": "Automic Vault", "url": SITE_ORIGIN + "/"},
            "about": tx(locale, "packageCatalogDescription", "Nucleus packages, AI agent package security, approval gates, and secret migration metadata"),
        },
    )


def render_hub_page(
    hub: PackageHub,
    pages: list[PackagePage],
    manifest: dict[str, Any],
    locale: dict[str, Any] | None = None,
) -> str:
    updated = fmt_date(manifest.get("generated_at", ""))
    top = pages[:72]
    secured = [page for page in pages if page.isotope]
    gated = [page for page in pages if page.approval_gate]
    rows = "\n".join(hub_package_row(page, locale) for page in top)
    description = short_text(
        tx(locale, "hubSchemaDescription", "{description} Browse {count} package pages with install commands, metadata, and Automic Vault security notes.", description=hub.description, count=len(pages)),
        155,
    )
    return html_doc(
        title=f"{hub.title} | Automic Vault package catalog",
        description=description,
        canonical=locale_url(hub.path, locale),
        alternates_path=hub.path,
        locale=locale,
        body=f"""
{nav('../../', locale)}
<main>
  <nav class="breadcrumbs" aria-label="Breadcrumbs">
    <a href="../../">{html_escape(tx(locale, 'home', 'Home'))}</a>
    <span>/</span>
    <a href="../">{html_escape(tx(locale, 'packages', 'Packages'))}</a>
    <span>/</span>
    <span>{html_escape(hub.title)}</span>
  </nav>
  <section class="pkg-hero pkg-hero-index" aria-labelledby="hub-title">
    <div class="hero-copy">
      <p class="eyebrow">{html_escape(hub.kicker)}</p>
      <h1 id="hub-title">{html_escape(hub.title)}</h1>
      <p class="lede">{html_escape(hub.description)}</p>
    </div>
    <aside class="hero-panel" aria-label="{attr(tx(locale, 'hubCounts', 'Hub counts'))}">
      {metric(tx(locale, 'packages', 'packages'), fmt_int(len(pages)))}
      {metric(tx(locale, 'radioisotopes', 'radioisotopes'), fmt_int(len(secured)))}
      {metric(tx(locale, 'approvalGates', 'approval gates'), fmt_int(len(gated)))}
      {metric(tx(locale, 'updated', 'updated'), updated)}
    </aside>
  </section>
  <section class="pkg-section split-section">
    <div>
      <p class="section-kicker">{html_escape(tx(locale, 'packageSummary', 'summary'))}</p>
      <h2>{html_escape(tx(locale, 'hubSummaryTitle', 'Why this package group is here'))}</h2>
      <p>{html_escape(hub_description_detail(hub, pages, locale))}</p>
    </div>
    <div class="detail-stack">
      <article>
        <h3>{html_escape(tx(locale, 'generatedSource', 'Generated source'))}</h3>
        <p>{html_escape(tx(locale, 'generatedSourceCopy', 'This hub uses the same local package data as individual package pages: Nucleus package metadata, Homebrew enrichment, Geiger classifier output, radioisotope manifests, and approval-gate seeds where available.'))}</p>
      </article>
      <article>
        <h3>{html_escape(tx(locale, 'hubReviewModel', 'Review model'))}</h3>
        <p>{html_escape(tx(locale, 'hubReviewCopy', 'Use the hub to find command families that need tighter secret injection, approval gates, or manual review before agents run them.'))}</p>
      </article>
    </div>
  </section>
  <section class="pkg-section">
    <p class="section-kicker">{html_escape(tx(locale, 'packages', 'packages'))}</p>
    <h2>{html_escape(tx(locale, 'hubIndexedPagesTitle', 'Indexed package pages'))}</h2>
    <div class="table-wrap hub-table">
      <table>
        <thead><tr><th>{html_escape(tx(locale, 'package', 'Package'))}</th><th>{html_escape(tx(locale, 'manager', 'Manager'))}</th><th>{html_escape(tx(locale, 'signals', 'Signals'))}</th><th>{html_escape(tx(locale, 'why', 'Why it appears here'))}</th></tr></thead>
        <tbody>{rows}</tbody>
      </table>
    </div>
  </section>
</main>
{footer('../../', locale)}
""",
        stylesheet_href=locale_path("/pkg/styles.css", locale),
        favicon_href="/favicon.ico",
        schema=schema_for_hub(hub, pages, description, updated, locale),
    )


def hub_description_detail(hub: PackageHub, pages: list[PackagePage], locale: dict[str, Any] | None = None) -> str:
    secured = sum(1 for page in pages if page.isotope)
    gated = sum(1 for page in pages if page.approval_gate)
    risked = sum(1 for page in pages if page.geiger and str(page.geiger.get("level") or "").lower() not in {"", "green", "low", "unknown"})
    return tx(
        locale,
        "hubDescription",
        "{title} currently includes {count} generated package pages. {secured} have radioisotope coverage, {gated} have approval-gate metadata, and {risked} have non-low Geiger classifier findings. The grouping comes from package metadata, so it can stay current as that metadata changes.",
        title=hub.title,
        count=len(pages),
        secured=secured,
        gated=gated,
        risked=risked,
    )


def hub_package_row(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    signals = []
    if page.isotope:
        signals.append("radioisotope")
    if page.approval_gate:
        signals.append(tx(locale, "approvalGatesKicker", "approval gate"))
    if page.geiger:
        signals.append(tx(locale, "riskLevel", "{level} risk", level=geiger_level_label(page.geiger)))
    if page.version:
        signals.append(f"v{page.version}")
    reason = hub_package_reason(page, locale)
    return (
        f'<tr><td><a href="{attr(page.path)}">{html_escape(page.display_name)}</a></td>'
        f"<td>{html_escape(package_manager_label(page))}</td>"
        f"<td>{html_escape(', '.join(signals) or label_for(page, locale))}</td>"
        f"<td>{html_escape(reason)}</td></tr>"
    )


def hub_package_reason(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    if page.isotope:
        title = (page.isotope.get("justification") or {}).get("title")
        if title:
            return str(title)
    if page.approval_gate:
        return tx(locale, "hubPackageReasonApproval", "{count} approval-gate rules are present.", count=page.approval_gate.get("rule_count") or "Local")
    if page.geiger:
        reasons = page.geiger.get("reasons") or []
        if reasons:
            return short_text(reasons[0], 140)
    if page.summary:
        return short_text(clean_summary(page.summary), 140)
    aliases = sorted(page.aliases)
    if aliases:
        return tx(locale, "hubPackageReasonAlias", "Executable aliases include {aliases}.", aliases=", ".join(aliases[:4]))
    return tx(locale, "hubPackageReasonDefault", "Matched package metadata for this hub.")


def schema_for_hub(hub: PackageHub, pages: list[PackagePage], description: str, updated: str, locale: dict[str, Any] | None = None) -> dict[str, Any]:
    url = locale_url(hub.path, locale)
    return {
        "@context": "https://schema.org",
        "@graph": [
            {"@type": "WebSite", "@id": f"{SITE_ORIGIN}/#website", "name": "Automic Vault", "url": f"{SITE_ORIGIN}/"},
            {"@type": "Organization", "@id": f"{SITE_ORIGIN}/#organization", "name": "Automic Vault", "url": f"{SITE_ORIGIN}/"},
            {"@type": "Person", "@id": f"{SITE_ORIGIN}/about/#max-howell", "name": "Max Howell", "url": f"{SITE_ORIGIN}/about/"},
            {
                "@type": "CollectionPage",
                "@id": f"{url}#webpage",
                "name": hub.title,
                "headline": hub.title,
                "url": url,
                "description": description,
                "inLanguage": (locale or {}).get("htmlLang") or "en",
                "dateModified": updated,
                "isPartOf": {"@id": f"{SITE_ORIGIN}/#website"},
                "about": {"@id": f"{SITE_ORIGIN}/#software"},
                "author": {"@id": f"{SITE_ORIGIN}/about/#max-howell"},
                "reviewedBy": {"@id": f"{SITE_ORIGIN}/about/#max-howell"},
                "publisher": {"@id": f"{SITE_ORIGIN}/#organization"},
                "mainEntity": {
                    "@type": "ItemList",
                    "numberOfItems": len(pages),
                    "itemListElement": [
                        {
                            "@type": "ListItem",
                            "position": index + 1,
                            "url": f"{SITE_ORIGIN}{page.path}",
                            "name": page.display_name,
                        }
                        for index, page in enumerate(pages[:50])
                    ],
                },
            },
            {
                "@type": "BreadcrumbList",
                "@id": f"{url}#breadcrumbs",
                "itemListElement": [
                    {"@type": "ListItem", "position": 1, "name": tx(locale, "home", "Home"), "item": locale_url("/", locale)},
                    {"@type": "ListItem", "position": 2, "name": tx(locale, "packages", "Packages"), "item": locale_url("/pkg/", locale)},
                    {"@type": "ListItem", "position": 3, "name": hub.title, "item": url},
                ],
            },
        ],
    }


def render_package_page(
    page: PackagePage,
    manifest: dict[str, Any],
    locale: dict[str, Any] | None = None,
) -> str:
    title = tx(
        locale,
        "installTitle",
        "Install {name} with {manager} | Automic Vault",
        name=page.display_name,
        manager=package_manager_label(page),
    )
    description = meta_description(page) if locale_code(locale) == "en" else tx(
        locale,
        "metaDescription",
        "Install {name} with {manager}. View executables, metadata, and security notes.",
        name=page.display_name,
        manager=package_manager_label(page),
    )
    updated = fmt_date(page.last_verified) or fmt_date(page.last_updated_at) or fmt_date(manifest.get("generated_at", ""))
    facts = package_facts(page, locale)
    install_section = render_concept_install(page, locale) if page.key == "brew:ripgrep" else render_install(page, locale)
    sections = [
        install_section,
        render_overview(page, locale),
        render_security(page, locale),
        render_executables(page, locale),
        render_freshness(page, manifest, locale),
        render_install_metadata(page, locale),
        render_related(page, locale),
        render_sources(page, locale),
    ]
    breadcrumbs = f"""
<nav class="breadcrumbs" aria-label="Breadcrumbs">
  <a href="../../../">{html_escape(tx(locale, 'home', 'Home'))}</a>
  <span>/</span>
  <a href="../../">{html_escape(tx(locale, 'packages', 'Packages'))}</a>
  <span>/</span>
  <span>{html_escape(page.display_name)}</span>
</nav>
"""
    return html_doc(
        title=title,
        description=description,
        canonical=locale_url(page.path, locale),
        alternates_path=page.path,
        locale=locale,
        robots="index,follow" if is_indexable_package_page(page) else "noindex,follow",
        body=f"""
{nav('../../../', locale)}
<main>
  {breadcrumbs}
  <section class="pkg-hero" aria-labelledby="pkg-title">
    <div class="hero-copy">
      <p class="eyebrow">{html_escape(tx(locale, 'packageIntelligence', '{provider} package intelligence', provider=page.provider))}</p>
      <h1 id="pkg-title">{html_escape(tx(locale, 'installHeading', 'Install {name}', name=page.display_name))}</h1>
      <p class="lede">{html_escape(localized_hero_sentence(page, locale))}</p>
      <div class="hero-actions">
        <a class="button primary" href="#install">{html_escape(tx(locale, 'installAction', 'Install command'))}</a>
        <a class="button secondary" href="#security">{html_escape(tx(locale, 'securityAction', 'Security notes'))}</a>
      </div>
    </div>
    <aside class="hero-panel" aria-label="{attr(tx(locale, 'packageFacts', 'Package facts'))}">
      {facts}
    </aside>
  </section>
  {''.join(sections)}
</main>
{footer('../../../', locale)}
""",
        stylesheet_href=locale_path("/pkg/styles.css", locale),
        favicon_href="/favicon.ico",
        schema=schema_for_package(page, description, updated, locale),
        extra_head=markdown_alternate_head(page, locale) if is_indexable_package_page(page) else "",
        extra_body=copy_script(locale),
    )


def render_concept_install(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    commands = install_command_entries(page)
    primary = commands[0] if commands else {
        "command": f"sudo av install {page.key}",
        "manager": "Automic Vault",
        "platform": "portable",
        "confidence": 1.0,
        "evidence": "deterministic local package key",
    }
    command = str(primary.get("command") or "")
    return f"""
<section id="install" class="pkg-section pkg-concept-install" aria-labelledby="install-title">
  <div class="pkg-concept-section-head">
      <p class="section-kicker">{html_escape(tx(locale, 'installRoutes', 'install routes'))}</p>
      <h2 id="install-title">{html_escape(tx(locale, 'installTitleConcept', 'Start with Vault, then choose the host package manager.'))}</h2>
      <p>{html_escape(tx(locale, 'installTitleConceptCopy', 'The page keeps the Automic Vault command first, then separates package-manager commands by operating system so the copy target is unambiguous.'))}</p>
  </div>
  <div class="pkg-concept-primary-command">
    <div class="pkg-concept-primary-head">
      <span>{html_escape(primary.get('manager') or 'Automic Vault')}</span>
      <button class="copy-button" type="button" data-copy="{attr(command)}" aria-label="{attr(tx(locale, 'copyInstallCommand', 'Copy install command'))}">{html_escape(tx(locale, 'copy', 'Copy'))}</button>
    </div>
    <pre><code>{html_escape(command)}</code></pre>
  </div>
  {render_concept_platforms(commands[1:], locale)}
</section>
"""


def render_concept_platforms(commands: list[dict[str, Any]], locale: dict[str, Any] | None = None) -> str:
    grouped: dict[str, list[dict[str, Any]]] = {"macos": [], "linux": [], "windows": [], "portable": []}
    for item in commands:
        platform = str(item.get("platform") or "portable")
        if platform not in grouped:
            platform = "portable"
        grouped[platform].append(item)
    labels = {
        "macos": "macOS",
        "linux": "Linux",
        "windows": "Windows",
        "portable": "Portable",
    }
    sections = []
    for index, platform in enumerate(("macos", "linux", "windows", "portable")):
        items = grouped.get(platform) or []
        if not items:
            continue
        rows = "".join(render_concept_command_row(item, locale) for item in items)
        count_label = tx(locale, "commandCount", "{count} commands", count=fmt_int(len(items)))
        sections.append(f"""
<article class="pkg-concept-platform" style="--i: {index}">
  <div class="pkg-concept-platform-head">
    <h3>{html_escape(labels[platform])}</h3>
    <span>{html_escape(count_label)}</span>
  </div>
  <div class="pkg-concept-command-list">{rows}</div>
</article>
""")
    if not sections:
        return f"<p>{html_escape(tx(locale, 'noPlatformCommands', 'No additional platform commands were present.'))}</p>"
    return f'<div class="pkg-concept-platform-grid" aria-label="{attr(tx(locale, "platformInstallCommands", "Platform install commands"))}">{"".join(sections)}</div>'


def render_concept_command_row(item: dict[str, Any], locale: dict[str, Any] | None = None) -> str:
    command_text = str(item.get("command") or "")
    manager = str(item.get("manager") or "shell")
    manager_label = install_command_manager_label(item)
    source_html = install_command_source_html(item, locale)
    try:
        confidence_value = float(item.get("confidence"))
    except (TypeError, ValueError):
        confidence_value = 0.0
    confidence_label = tx(locale, "verified", "verified") if confidence_value >= 0.9 else tx(locale, "inferred", "inferred")
    return f"""
<div class="pkg-concept-command-row">
  <div class="install-command-head">
    <strong class="install-command-eyebrow">{html_escape(manager_label)}</strong>
    <span>{html_escape(confidence_label)} / {html_escape(f'{confidence_value:.0%}')}</span>
  </div>
  <div class="install-command-shell">
    <code>{html_escape(command_text)}</code>
    <button class="copy-button" type="button" data-copy="{attr(command_text)}" aria-label="{attr(tx(locale, 'copyManagerInstallCommand', 'Copy {manager} install command', manager=manager_label))}">{html_escape(tx(locale, 'copy', 'Copy'))}</button>
  </div>
  {source_html}
</div>
"""


def markdown_alternate_head(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    return f'  <link rel="alternate" type="text/markdown" href="{attr(f"{locale_url(page.path, locale)}index.md")}">'


def render_package_markdown(
    page: PackagePage,
    manifest: dict[str, Any],
    locale: dict[str, Any] | None = None,
) -> str:
    updated = fmt_date(page.last_verified) or fmt_date(page.last_updated_at) or fmt_date(manifest.get("generated_at", ""))
    lines = [
        f"# {md_text(tx(locale, 'installHeading', 'Install {name}', name=page.display_name))}",
        "",
        md_text(localized_hero_sentence(page, locale)),
        "",
        f"## {md_text(tx(locale, 'install', 'Install'))}",
        "",
        "```sh",
        install_command(page),
        "```",
        "",
    ]
    lines.extend(md_install_command_groups(page, locale))
    lines.extend([f"## {md_text(tx(locale, 'packageFacts', 'Package Facts'))}", ""])
    fact_rows = [
        (tx(locale, "packageKey", "Package key"), page.key),
        (tx(locale, "packageManager", "Package manager"), package_manager_label(page)),
        (tx(locale, "packageManagerPage", "Package manager URL"), page.package_manager_url),
        (tx(locale, "version", "Version"), page.version),
        (tx(locale, "sourceSummary", "Source summary"), clean_summary(page.summary)),
        (tx(locale, "homepage", "Homepage"), page.homepage),
        (tx(locale, "repository", "Repository"), page.repository),
        (tx(locale, "upstreamDocs", "Upstream docs"), page.upstream_docs),
        (tx(locale, "license", "License"), page.license),
        (tx(locale, "sourceArchive", "Source archive"), page.source_archive),
        (tx(locale, "issueTracker", "Issue tracker"), page.issue_tracker),
        (tx(locale, "published", "Published"), page.published_at),
        (tx(locale, "verified", "Last verified"), page.last_verified),
        (tx(locale, "updated", "Last updated"), page.last_updated_at),
        (tx(locale, "generatedSource", "Generated"), manifest.get("generated_at")),
    ]
    lines.extend(md_fact_lines(fact_rows))
    lines.extend(md_section_list(tx(locale, "executables", "Executables"), executable_markdown_items(page, locale)))
    lines.extend(md_section_list(tx(locale, "dependencies", "Dependencies"), [*page.dependencies]))
    lines.extend(md_section_list(tx(locale, "buildDependencies", "Build Dependencies"), [*page.build_dependencies]))
    lines.extend(md_section_list(tx(locale, "usesFromMacos", "macOS Provided Libraries"), [*page.uses_from_macos]))
    lines.extend(md_install_behavior_section(page, locale))
    lines.extend(md_freshness_section(page, manifest, locale))
    lines.extend(md_security_section(page, locale))
    lines.extend(md_related_section(page, locale))
    lines.extend(md_section_list(tx(locale, "sources", "Sources"), page.source_notes))
    return "\n".join(lines).rstrip() + "\n"


def md_install_command_groups(page: PackagePage, locale: dict[str, Any] | None = None) -> list[str]:
    commands = install_command_entries(page)[1:]
    if not commands:
        return []
    labels = {
        "macos": "macOS",
        "linux": "Linux",
        "windows": "Windows",
        "portable": "Portable and language managers",
    }
    grouped: dict[str, list[dict[str, Any]]] = {"macos": [], "linux": [], "windows": [], "portable": []}
    for item in commands:
        platform = str(item.get("platform") or "portable")
        if platform not in grouped:
            platform = "portable"
        grouped[platform].append(item)
    lines: list[str] = [f"{md_text(tx(locale, 'additionalInstallCommands', 'Additional install commands'))}:", ""]
    for platform in ("macos", "linux", "windows", "portable"):
        items = grouped.get(platform) or []
        if not items:
            continue
        lines.extend([f"### {labels[platform]}", ""])
        for item in items:
            manager = md_text(item.get("manager") or "shell")
            command_text = md_text(item.get("command") or "")
            try:
                confidence_text = f"{float(item.get('confidence')):.0%}"
            except (TypeError, ValueError):
                confidence_text = tx(locale, "unknownConfidence", "unknown confidence")
            evidence = md_text(item.get("evidence") or "")
            lines.extend([f"- {manager} ({confidence_text}):", "", "```sh", command_text, "```"])
            if evidence:
                lines.extend(["", f"  {md_text(tx(locale, 'markdownEvidence', 'Evidence'))}: {evidence}"])
            lines.append("")
    return lines


def md_fact_lines(rows: list[tuple[str, Any]]) -> list[str]:
    lines: list[str] = []
    for label, value in rows:
        if value:
            lines.append(f"- **{md_text(label)}:** {md_value(value)}")
    lines.append("")
    return lines


def md_section_list(title: str, items: list[Any]) -> list[str]:
    values = [md_value(item) for item in items if str(item or "").strip()]
    if not values:
        return []
    lines = [f"## {md_text(title)}", ""]
    lines.extend(f"- {value}" for value in values)
    lines.append("")
    return lines


def executable_markdown_items(page: PackagePage, locale: dict[str, Any] | None = None) -> list[str]:
    items: list[str] = []
    for item in page.executables:
        if not isinstance(item, dict):
            continue
        name = str(item.get("name") or item.get("target") or item.get("source") or "").strip()
        if not name:
            continue
        kind = str(item.get("kind") or item.get("type") or tx(locale, "executable", "executable")).strip()
        note = str(item.get("note") or item.get("source") or "").strip()
        detail = f"{name} ({kind})"
        if note:
            detail += f": {note}"
        items.append(detail)
    for binary in page.binaries:
        if not isinstance(binary, dict):
            continue
        name = str(binary.get("target") or binary.get("source") or "").strip()
        if name:
            items.append(f"{name} ({tx(locale, 'binary', 'binary')})")
    for alias_name in sorted(page.aliases):
        items.append(f"{alias_name} ({tx(locale, 'alias', 'alias')})")
    return items


def md_install_behavior_section(page: PackagePage, locale: dict[str, Any] | None = None) -> list[str]:
    behavior = page.install_behavior or {}
    items: list[str] = []
    if behavior.get("postInstallDefined") is not None:
        items.append(f"{tx(locale, 'postInstallHook', 'Post-install hook')}: {tx(locale, 'defined', 'defined') if behavior.get('postInstallDefined') else tx(locale, 'notDefined', 'not defined')}")
    if behavior.get("service"):
        items.append(f"{tx(locale, 'service', 'Service')}: {behavior.get('service')}")
    if behavior.get("caveats"):
        items.append(f"{tx(locale, 'caveats', 'Caveats')}: {behavior.get('caveats')}")
    if behavior.get("lifecycleScripts"):
        items.append(f"{tx(locale, 'lifecycleScripts', 'Lifecycle scripts')}: {', '.join(str(item) for item in behavior.get('lifecycleScripts') or [])}")
    if behavior.get("pythonRequires"):
        items.append(f"{tx(locale, 'pythonRequires', 'Python requires')}: {behavior.get('pythonRequires')}")
    if behavior.get("requiresDistCount") is not None:
        items.append(f"{tx(locale, 'pypiDependencySpecs', 'PyPI dependency specs')}: {behavior.get('requiresDistCount')}")
    bottle = page.bottle or {}
    if bottle:
        available = bottle.get("available")
        bottle_detail = tx(locale, "available", "available") if available else tx(locale, "notAvailable", "not available")
        platforms = bottle.get("platforms") or []
        if platforms:
            bottle_detail += f" {tx(locale, 'onPlatforms', 'on')} {', '.join(str(item) for item in platforms[:12])}"
        items.append(f"{tx(locale, 'bottle', 'Bottle')}: {bottle_detail}")
    return md_section_list(tx(locale, "installBehavior", "Install Behavior"), items)


def md_freshness_section(page: PackagePage, manifest: dict[str, Any], locale: dict[str, Any] | None = None) -> list[str]:
    freshness = page.version_freshness or {}
    manager = freshness.get("packageManager") if isinstance(freshness.get("packageManager"), dict) else {}
    site = freshness.get("siteData") if isinstance(freshness.get("siteData"), dict) else {}
    upstream = freshness.get("upstream") if isinstance(freshness.get("upstream"), dict) else {}
    warnings = freshness.get("warnings") if isinstance(freshness.get("warnings"), list) else []
    items = [
        f"{tx(locale, 'pageGenerated', 'Page generated')}: {fmt_date(manifest.get('generated_at', '')) or tx(locale, 'unknown', 'unknown')}",
        f"{tx(locale, 'managerVersion', 'Package-manager version')}: {manager.get('version') or page.version or tx(locale, 'unknown', 'unknown')}",
    ]
    if manager.get("updatedAt"):
        items.append(f"{tx(locale, 'managerUpdated', 'Package-manager updated')}: {fmt_date(manager.get('updatedAt')) or manager.get('updatedAt')}")
    if site.get("status"):
        items.append(f"{tx(locale, 'localData', 'Local data status')}: {site.get('status')}")
    if upstream.get("repository"):
        items.append(f"{tx(locale, 'upstreamRepository', 'Upstream repository')}: {upstream.get('repository')}")
    if upstream.get("latestVersion"):
        items.append(f"{tx(locale, 'upstreamLatestDetected', 'Upstream latest detected')}: {upstream.get('latestVersion')} ({upstream.get('comparison') or tx(locale, 'unknown', 'unknown')})")
    for item in warnings[:8]:
        if isinstance(item, dict):
            items.append(f"{item.get('severity', 'info')}: {item.get('message')}")
    return md_section_list(tx(locale, "freshness", "Freshness"), items)


def md_security_section(page: PackagePage, locale: dict[str, Any] | None = None) -> list[str]:
    lines = [f"## {md_text(tx(locale, 'securityNotes', 'Security Notes'))}", "", md_text(security_summary(page, locale)), ""]
    if page.geiger:
        lines.append(f"- **{md_text(tx(locale, 'geigerRisk', 'Geiger risk'))}:** {md_text(geiger_level_label(page.geiger))} / {md_text(geiger_confidence_label(page.geiger))}")
        for reason in (page.geiger.get("reasons") or [])[:5]:
            lines.append(f"- {md_value(reason)}")
    if page.isotope:
        justification = page.isotope.get("justification") or {}
        title = justification.get("title") or tx(locale, "radioisotopeCoverage", "Radioisotope coverage")
        lines.append(f"- **{md_text(tx(locale, 'radioisotopeKicker', 'Radioisotope'))}:** {md_value(title)}")
    if page.approval_gate:
        lines.append(f"- **{md_text(tx(locale, 'approvalRules', 'Approval gate rules'))}:** {md_value(page.approval_gate.get('rule_count'))}")
    lines.append("")
    return lines


def md_related_section(page: PackagePage, locale: dict[str, Any] | None = None) -> list[str]:
    related = list(page.related_packages) + list(page.also_available_via)
    if not related and not page.package_hubs:
        return []
    lines = [f"## {md_text(tx(locale, 'relatedLinks', 'Related Links'))}", ""]
    for item in related[:24]:
        if not isinstance(item, dict):
            continue
        provider = str(item.get("provider") or "").strip()
        name = str(item.get("name") or "").strip()
        label = str(item.get("label") or name).strip()
        if not provider or not name:
            continue
        reason = str(item.get("reason") or item.get("rel") or "").strip()
        href = f"{SITE_ORIGIN}/pkg/{provider}/{slugify(name)}/"
        suffix = f" - {md_text(reason)}" if reason else ""
        lines.append(f"- [{md_text(label)}]({href}){suffix}")
    for hub in page.package_hubs[:12]:
        if not isinstance(hub, dict):
            continue
        slug = str(hub.get("slug") or "").strip()
        label = str(hub.get("label") or slug).strip()
        if slug and label:
            lines.append(f"- [{md_text(label)}]({SITE_ORIGIN}/pkg/{slug}/)")
    lines.append("")
    return lines


def md_value(value: Any) -> str:
    text = md_text(value)
    if re.match(r"^https?://", text):
        return f"<{text}>"
    return text


def md_text(value: Any) -> str:
    return normalize_space(value).replace("|", "\\|")


def hero_sentence(page: PackagePage) -> str:
    summary = clean_summary(page.summary)
    if summary and install_command(page):
        alternate = alternate_install_sentence(page)
        alternate_text = f" {alternate}" if alternate else ""
        return f"{sentence_text(summary)} Version {page.version or 'unknown'} via {package_manager_label(page)}; verified {fmt_date(page.last_verified) or fmt_date(page.last_updated_at) or 'from local package data'}.{alternate_text}"
    if page.isotope:
        title = ((page.isotope.get("justification") or {}).get("title") or "secret handling").rstrip(".")
        return f"Automic Vault tracks {page.display_name} because {title.lower()} affects agent-run command-line tools on macOS."
    if page.approval_gate:
        return f"Automic Vault has approval-gate metadata for {page.display_name}, including high-risk commands and recommended human review points."
    if summary:
        return f"Nucleus can resolve {page.display_name}: {summary}"
    return f"Nucleus package metadata for {page.display_name}, from local Automic Vault package sources."


def localized_hero_sentence(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    if locale_code(locale) == "en":
        return hero_sentence(page)
    return tx(
        locale,
        "heroSentence",
        "View install routes, executables, metadata, and security notes for {name}.",
        name=page.display_name,
    )


def sentence_text(value: str) -> str:
    text = normalize_space(value)
    if not text:
        return ""
    if text[-1] not in ".!?":
        text += "."
    return text


def meta_description(page: PackagePage) -> str:
    alternate = alternate_install_command(page)
    if alternate:
        parts = [
            (
                f"Install {page.display_name} with {package_manager_label(page)} "
                f"or {alternate.get('manager')}: {alternate.get('command')}."
            )
        ]
    else:
        parts = [f"Install {page.display_name} with {package_manager_label(page)}."]
    summary = clean_summary(page.summary)
    if summary:
        parts.append(summary)
    if page.executables or page.aliases:
        parts.append("View executables, metadata, and security notes.")
    if page.isotope:
        title = (page.isotope.get("justification") or {}).get("title")
        if title:
            parts.append(f"Radioisotope coverage: {title}.")
    if page.approval_gate:
        parts.append(f"Includes {page.approval_gate.get('rule_count')} approval-gate rules.")
    return short_text(" ".join(parts), 155)


def alternate_install_command(page: PackagePage) -> dict[str, Any] | None:
    for item in install_command_entries(page):
        if not isinstance(item, dict) or item.get("kind") == "automic_vault":
            continue
        command_key = native_command_package_key(str(item.get("command") or ""))
        if command_key is None:
            continue
        provider, _ = command_key
        if provider != page.provider:
            return item
    return None


def alternate_install_sentence(page: PackagePage) -> str:
    alternate = alternate_install_command(page)
    if not alternate:
        return ""
    manager = str(alternate.get("manager") or "another package manager")
    command_text = str(alternate.get("command") or "").strip()
    if not command_text:
        return ""
    return f"Also installable with {manager}: {command_text}."


def label_for(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    labels = [page.provider]
    if page.isotope:
        labels.append(tx(locale, "radioisotopeKicker", "radioisotope"))
    if page.approval_gate:
        labels.append(tx(locale, "approvalGatesKicker", "approval gates"))
    rank = page.popularity.get("rank")
    if rank:
        labels.append(f"{tx(locale, 'rank', 'rank')} {rank}")
    return " / ".join(labels)


def package_facts(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    facts = [metric(tx(locale, "manager", "manager"), package_manager_label(page))]
    if page.version:
        facts.append(metric(tx(locale, "version", "version"), page.version))
    if page.license:
        facts.append(metric(tx(locale, "license", "license"), page.license))
    if page.geiger:
        facts.append(metric(tx(locale, "risk", "risk"), geiger_level_label(page.geiger)))
        facts.append(metric(tx(locale, "classifierConfidence", "classifier confidence"), geiger_confidence_label(page.geiger)))
    rank = page.popularity.get("rank")
    if rank:
        facts.append(metric(tx(locale, "rank", "rank"), fmt_int(rank)))
    installs = page.popularity.get("installs_per_365_days") or page.popularity.get("downloads_per_30_days")
    if installs:
        label = tx(locale, "installs365d", "365d installs") if page.popularity.get("installs_per_365_days") else tx(locale, "downloads30d", "30d downloads")
        facts.append(metric(label, fmt_int(installs)))
    if page.isotope:
        facts.append(metric(tx(locale, "radioisotopeKicker", "radioisotope"), tx(locale, "covered", "covered")))
    if page.approval_gate:
        facts.append(metric(tx(locale, "approvalRules", "approval rules"), fmt_int(page.approval_gate.get("rule_count"))))
    if page.last_verified:
        facts.append(metric(tx(locale, "verified", "verified"), fmt_date(page.last_verified)))
    elif page.last_updated_at:
        facts.append(metric(tx(locale, "updated", "updated"), fmt_date(page.last_updated_at)))
    return "".join(facts)


def metric(label: str, value: Any) -> str:
    return f'<div class="metric"><span>{html_escape(label)}</span><strong>{html_escape(value)}</strong></div>'


def package_manager_label(page: PackagePage) -> str:
    if page.package_manager:
        return page.package_manager
    return {
        "brew": "Homebrew",
        "cask": "Homebrew Cask",
        "npm": "npm",
        "pip": "PyPI",
    }.get(page.provider, page.provider)


def install_command(page: PackagePage) -> str:
    commands = install_command_entries(page)
    if commands:
        return str(commands[0].get("command") or "")
    return f"sudo av install {page.key}"


def native_install_command(page: PackagePage) -> str:
    if page.provider == "brew":
        return f"brew install {page.name}"
    if page.provider == "cask":
        return f"brew install --cask {page.name}"
    if page.provider == "npm":
        return f"npm install -g {page.name}"
    if page.provider == "pip":
        return f"pip install {page.name}"
    return ""


def install_command_entries(page: PackagePage) -> list[dict[str, Any]]:
    if page.install_commands:
        return page.install_commands
    entries = [
        {
            "platform": "portable",
            "manager": "Automic Vault",
            "command": f"sudo av install {page.key}",
            "kind": "automic_vault",
            "confidence": 1.0,
            "evidence": "deterministic local package key",
        }
    ]
    native = native_install_command(page)
    if native:
        platform = "macos" if page.provider in {"brew", "cask"} else "portable"
        manager = package_manager_label(page)
        entries.append({
            "platform": platform,
            "manager": manager,
            "command": native,
            "kind": "package_manager",
            "confidence": 1.0,
            "evidence": f"{manager} package metadata",
        })
    return entries


def source_backed_schema_commands(page: PackagePage) -> list[dict[str, Any]]:
    result = []
    for item in install_command_entries(page):
        command_text = str(item.get("command") or "").strip()
        evidence = str(item.get("evidence") or "")
        if not command_text or "agent-inferred" in evidence:
            continue
        try:
            confidence = float(item.get("confidence"))
        except (TypeError, ValueError):
            confidence = 0.0
        if confidence < 0.9:
            continue
        result.append(item)
    return result


def geiger_level_label(geiger: dict[str, Any]) -> str:
    level = geiger.get("level") or "unknown"
    return str(level)


def geiger_confidence_label(geiger: dict[str, Any]) -> str:
    confidence = geiger.get("confidence") or ""
    return str(confidence or "unknown")


def render_install(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    commands = install_command_entries(page)
    primary = commands[0] if commands else {
        "command": f"sudo av install {page.key}",
        "manager": "Automic Vault",
        "platform": "portable",
        "confidence": 1.0,
        "evidence": "deterministic local package key",
    }
    command = str(primary.get("command") or "")
    notes = page.install.get("notes") or []
    note_items = "".join(f"<li>{html_escape(note)}</li>" for note in notes[:6])
    manager = page.package_manager_url
    manager_link = (
        f'<a href="{attr(manager)}">{html_escape(manager)}</a>'
        if manager
        else html_escape(tx(locale, "installSourceMissing", "{manager} metadata was not linked in local data.", manager=package_manager_label(page)))
    )
    platform_html = render_platform_install_commands(commands[1:], locale)
    return f"""
<section id="install" class="pkg-section install-section" aria-labelledby="install-title">
  <div class="install-command-panel">
    <div>
      <p class="section-kicker">{html_escape(tx(locale, 'install', 'install'))}</p>
      <h2 id="install-title">{html_escape(tx(locale, 'automicVaultInstallHeading', 'Install with Automic Vault'))}</h2>
    </div>
    <div class="terminal-block">
      <div class="terminal-head">
        <span>{html_escape(primary.get('manager') or 'shell')}</span>
        <div class="terminal-actions">
          <a class="download-av-button" href="/download/" aria-label="{attr(tx(locale, 'downloadAV', 'Download Automic Vault'))}">{html_escape(tx(locale, 'downloadAV', 'Download AV'))}</a>
          <button class="copy-button" type="button" data-copy="{attr(command)}" aria-label="{attr(tx(locale, 'copyInstallCommand', 'Copy install command'))}">{html_escape(tx(locale, 'copy', 'Copy'))}</button>
        </div>
      </div>
      <pre><code>{html_escape(command)}</code></pre>
    </div>
    {platform_html}
  </div>
  <div class="install-notes-grid">
    <article>
      <h3>{html_escape(tx(locale, 'packageManagerSource', 'Package manager source'))}</h3>
      <p>{manager_link}</p>
    </article>
    <article>
      <h3>{html_escape(tx(locale, 'platformNotes', 'Platform notes'))}</h3>
      <ul>{note_items or f'<li>{html_escape(tx(locale, "noPlatformNotes", "No package-specific platform notes were present."))}</li>'}</ul>
    </article>
  </div>
</section>
"""


def render_platform_install_commands(commands: list[dict[str, Any]], locale: dict[str, Any] | None = None) -> str:
    grouped: dict[str, list[dict[str, Any]]] = {"macos": [], "linux": [], "windows": [], "portable": []}
    for item in commands:
        platform = str(item.get("platform") or "portable")
        if platform not in grouped:
            platform = "portable"
        grouped[platform].append(item)
    labels = {
        "macos": "macOS",
        "linux": "Linux",
        "windows": "Windows",
        "portable": "Portable and language managers",
    }
    sections = []
    for platform in ("macos", "linux", "windows", "portable"):
        items = grouped.get(platform) or []
        if not items:
            continue
        rows = "".join(install_command_row(item, locale) for item in items)
        sections.append(f"""
<article>
  <h3>{html_escape(labels[platform])}</h3>
  <div class="install-command-list">{rows}</div>
</article>
""")
    if not sections:
        return ""
    return f"""
<div class="platform-install-grid" aria-label="{attr(tx(locale, 'platformInstallCommands', 'Platform install commands'))}">
  {''.join(sections)}
</div>
"""


def install_command_row(item: dict[str, Any], locale: dict[str, Any] | None = None) -> str:
    command_text = str(item.get("command") or "")
    manager = str(item.get("manager") or "shell")
    manager_label = install_command_manager_label(item)
    source_html = install_command_source_html(item, locale)
    confidence = item.get("confidence")
    try:
        confidence_value = float(confidence)
    except (TypeError, ValueError):
        confidence_value = 0.0
    confidence_label = tx(locale, "verified", "verified") if confidence_value >= 0.9 else tx(locale, "inferred", "inferred")
    return f"""
<div class="install-command-row">
  <div class="install-command-head">
    <strong class="install-command-eyebrow">{html_escape(manager_label)}</strong>
    <span>{html_escape(confidence_label)} · {html_escape(f'{confidence_value:.0%}')}</span>
  </div>
  <div class="install-command-shell">
    <code>{html_escape(command_text)}</code>
    <button class="copy-button" type="button" data-copy="{attr(command_text)}" aria-label="{attr(tx(locale, 'copyManagerInstallCommand', 'Copy {manager} install command', manager=manager_label))}">{html_escape(tx(locale, 'copy', 'Copy'))}</button>
  </div>
  {source_html}
</div>
"""


def install_command_manager_label(item: dict[str, Any]) -> str:
    manager = str(item.get("manager") or "shell").strip()
    source = item.get("source")
    source_manager = ""
    if isinstance(source, dict):
        source_manager = str(source.get("manager") or "").strip()
    manager_key = (source_manager or manager).lower()
    labels = {
        "apk": "Alpine Linux apk",
        "apt": "Debian apt",
        "chocolatey": "Chocolatey",
        "dnf": "Fedora dnf",
        "macports": "MacPorts",
        "nix": "Nix",
        "pacman": "Arch Linux pacman",
        "scoop": "Scoop",
        "winget": "Windows Package Manager",
        "zypper": "openSUSE zypper",
        "npm": "npm",
        "pip": "Python pip",
    }
    return labels.get(manager_key, manager or "shell")


def install_command_source_html(item: dict[str, Any], locale: dict[str, Any] | None = None) -> str:
    source = item.get("source")
    if isinstance(source, dict):
        label = str(source.get("source_label") or "").strip()
        package_name = str(source.get("package_name") or source.get("package_id") or "").strip()
        source_url = str(source.get("source_url") or "").strip()
        pieces = [html_escape(piece) for piece in (label, package_name) if piece]
        text = " · ".join(pieces)
        if source_url:
            link_label = source_host_label(source_url)
            link = f'<a href="{attr(source_url)}" aria-label="{attr(tx(locale, "sourceDatabaseAria", "Open source database on {host}", host=link_label))}">{html_escape(tx(locale, "installSourcePrefix", "source"))}: {html_escape(link_label)}</a>'
            text = f"{text} · {link}" if text else link
        return f'<p class="install-command-source">{text}</p>' if text else ""
    evidence = str(item.get("evidence") or "").strip()
    manager = str(item.get("manager") or "").strip()
    if evidence == f"{manager} package metadata":
        evidence = tx(locale, "managerPackageMetadata", "{manager} package metadata", manager=manager)
    return f'<p class="install-command-source">{html_escape(evidence)}</p>' if evidence else ""


def source_host_label(url: str) -> str:
    try:
        parsed = urllib.parse.urlparse(url)
    except ValueError:
        return "source"
    host = parsed.netloc.strip()
    return host or "source"


def render_overview(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    aliases = sorted(page.aliases)[:32]
    alias_html = "".join(f"<li>{html_escape(alias)}</li>" for alias in aliases)
    alias_block = f"<ul class=\"chip-list\">{alias_html}</ul>" if aliases else f"<p>{html_escape(tx(locale, 'noAliases', 'No executable aliases were found in the local package database.'))}</p>"
    homepage = f'<a href="{attr(page.homepage)}">{html_escape(page.homepage)}</a>' if page.homepage else html_escape(tx(locale, "homepageMissing", "Not present in the local metadata."))
    source_summary = clean_summary(page.summary)
    summary = html_escape(source_summary if locale_code(locale) == "en" else tx(locale, "summaryFallback", "Automic Vault publishes package-specific install routes, executable facts, and security metadata for {name} from local package data.", name=page.display_name))
    source_summary_html = ""
    if locale_code(locale) != "en" and source_summary:
        source_summary_html = f"""
    <article>
      <h3>{html_escape(tx(locale, 'sourceSummary', 'Source summary'))}</h3>
      <p>{html_escape(source_summary)}</p>
    </article>
"""
    return f"""
<section class="pkg-section split-section">
  <div>
    <p class="section-kicker">{html_escape(tx(locale, 'overview', 'overview'))}</p>
    <h2>{html_escape(tx(locale, 'packageSummary', 'Package summary'))}</h2>
    <p>{summary}</p>
  </div>
  <div class="detail-stack">
    <article>
      <h3>{html_escape(tx(locale, 'homepage', 'Homepage'))}</h3>
      <p>{homepage}</p>
    </article>
    <article>
      <h3>{html_escape(tx(locale, 'commandsAndAliases', 'Commands and aliases'))}</h3>
      {alias_block}
    </article>
    {source_summary_html}
  </div>
</section>
"""


def render_security(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    geiger = render_geiger(page, locale)
    install_signals = render_install_behavior_signals(page, locale)
    if page.isotope:
        justification = page.isotope.get("justification") or {}
        title = html_escape(justification.get("title") or tx(locale, "radioisotopeCoverage", "Radioisotope coverage"))
        detail = html_escape(paragraph_text(justification.get("detail") or page.isotope_readme or tx(locale, "radioisotopeManifestFallback", "Automic Vault has a local radioisotope manifest for this package.")))
        caveats = page.isotope.get("caveats") or []
        caveat_items = "".join(f"<li>{html_escape(item)}</li>" for item in caveats[:8])
        readme = render_readme_excerpt(page, locale)
        release = page.isotope.get("releaseUrl") or ""
        release_html = f'<a href="{attr(release)}">{html_escape(release)}</a>' if release else html_escape(tx(locale, "localRadioisotopeManifest", "Local radioisotope manifest"))
        return f"""
<section id="security" class="pkg-section security-section">
  <div>
    <p class="section-kicker">{html_escape(tx(locale, 'radioisotopeKicker', 'radioisotope'))}</p>
    <h2>{title}</h2>
    <p>{detail}</p>
    {geiger}
    {install_signals}
    {readme}
  </div>
  <div class="detail-stack">
    <article>
      <h3>{html_escape(tx(locale, 'coverageSource', 'Coverage source'))}</h3>
      <p>{release_html}</p>
    </article>
    <article>
      <h3>{html_escape(tx(locale, 'caveats', 'Caveats'))}</h3>
      <ul>{caveat_items or f'<li>{html_escape(tx(locale, "noCaveats", "No caveats were listed in the local manifest."))}</li>'}</ul>
    </article>
  </div>
</section>
{render_gate(page, locale)}
"""
    return render_gate(page, locale) or f"""
<section id="security" class="pkg-section security-section">
  <div>
    <p class="section-kicker">{html_escape(tx(locale, 'securityPosture', 'security posture'))}</p>
    <h2>{html_escape(security_heading(page, locale))}</h2>
    <p>{html_escape(security_summary(page, locale))}</p>
    {geiger}
    {install_signals}
  </div>
  <div class="detail-stack">
    <article>
      <h3>{html_escape(tx(locale, 'recommendedReview', 'Recommended review'))}</h3>
      <p>{html_escape(tx(locale, 'recommendedReviewCopy', 'Before unattended agent use, check whether the tool reads plaintext credentials, writes remote state, publishes artifacts, or shells out to plugins.'))}</p>
    </article>
  </div>
</section>
"""


def security_heading(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    if page.geiger:
        return tx(locale, "riskLevel", "Risk level: {level}", level=geiger_level_label(page.geiger))
    return tx(locale, "radioisotopeMissingHeading", "No radioisotope coverage found yet")


def security_summary(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    if page.geiger:
        reasons = page.geiger.get("reasons") or []
        if reasons:
            return " ".join(str(reason).rstrip(".") + "." for reason in reasons[:2])
    return tx(
        locale,
        "radioisotopeMissingSummary",
        "No matching local radioisotope manifest was found for {name}. Nucleus package metadata is still published here so future coverage has a stable package URL.",
        name=page.display_name,
    )


def render_geiger(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    if not page.geiger:
        return ""
    reasons = "".join(f"<li>{html_escape(reason)}</li>" for reason in (page.geiger.get("reasons") or [])[:5])
    signals = "".join(f"<li>{html_escape(signal)}</li>" for signal in (page.geiger.get("signals") or [])[:5])
    return f"""
<div class="signal-grid" aria-label="{attr(tx(locale, 'geigerSignalsAria', 'Geiger classifier signals'))}">
  <article>
    <h3>{html_escape(tx(locale, 'riskClassifier', 'Risk classifier'))}</h3>
    <p><strong>{html_escape(geiger_level_label(page.geiger))}</strong> {html_escape(tx(locale, 'risk', 'risk'))} · {html_escape(geiger_confidence_label(page.geiger))} {html_escape(tx(locale, 'confidence', 'confidence'))} · {html_escape(page.geiger.get('category') or tx(locale, 'uncategorized', 'uncategorized'))}</p>
  </article>
  <article>
    <h3>{html_escape(tx(locale, 'why', 'Why'))}</h3>
    <ul>{reasons or f'<li>{html_escape(tx(locale, "noClassifierReasons", "No classifier reasons were present."))}</li>'}</ul>
  </article>
  <article>
    <h3>{html_escape(tx(locale, 'signals', 'Signals'))}</h3>
    <ul>{signals or f'<li>{html_escape(tx(locale, "noClassifierSignals", "No classifier signals were present."))}</li>'}</ul>
  </article>
</div>
"""


def render_install_behavior_signals(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    signals: list[str] = []
    behavior = page.install_behavior or {}
    lifecycle = behavior.get("lifecycleScripts") or []
    if lifecycle:
        signals.append(tx(locale, "signalNpmLifecycle", "npm lifecycle scripts are declared: {scripts}.", scripts=", ".join(str(item) for item in lifecycle[:5])))
    if behavior.get("postInstallDefined") is True:
        label = tx(locale, "signalNpmPostinstall", "npm package metadata declares a postinstall script.") if page.provider == "npm" else tx(locale, "signalHomebrewPostinstall", "Homebrew declares a post-install hook for this formula.")
        signals.append(label)
    elif behavior.get("postInstallDefined") is False:
        label = tx(locale, "signalNoNpmPostinstall", "No npm postinstall script is recorded in package metadata.") if page.provider == "npm" else tx(locale, "signalNoHomebrewPostinstall", "No Homebrew post-install hook is recorded in formula metadata.")
        signals.append(label)
    if behavior.get("prepareDefined") is True:
        signals.append(tx(locale, "signalNpmPrepare", "npm package metadata declares a prepare script."))
    if behavior.get("pythonRequires"):
        signals.append(tx(locale, "signalPythonRequires", "PyPI metadata requires Python {version}.", version=behavior.get("pythonRequires")))
    if behavior.get("requiresDistCount"):
        signals.append(tx(locale, "signalRequiresDistCount", "PyPI metadata lists {count} dependency specifications.", count=behavior.get("requiresDistCount")))
    if behavior.get("service"):
        signals.append(tx(locale, "signalService", "Formula metadata declares a service or daemon block."))
    if page.bottle:
        if page.bottle.get("available"):
            platforms = page.bottle.get("platforms") or []
            if platforms:
                signals.append(tx(locale, "signalBottlePlatforms", "Homebrew bottle metadata is available for {count} platform targets.", count=len(platforms)))
            else:
                signals.append(tx(locale, "signalBottleAvailable", "Homebrew bottle metadata is available."))
        else:
            signals.append(tx(locale, "signalNoBottle", "No Homebrew bottle metadata was recorded."))
    if page.dependencies:
        signals.append(tx(locale, "signalDependencies", "Installs with {count} runtime dependencies.", count=len(page.dependencies)))
    if page.build_dependencies:
        signals.append(tx(locale, "signalBuildDependencies", "Build metadata lists {count} build dependencies.", count=len(page.build_dependencies)))
    if not signals:
        return ""
    items = "".join(f"<li>{html_escape(signal)}</li>" for signal in signals[:6])
    return f"""
<div class="signal-grid install-signal-grid" aria-label="{attr(tx(locale, 'installBehaviorSignals', 'Install behavior signals'))}">
  <article>
    <h3>{html_escape(tx(locale, 'installBehaviorTitle', 'Install behavior'))}</h3>
    <ul>{items}</ul>
  </article>
</div>
"""


def render_readme_excerpt(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    if not page.isotope_readme_html:
        return ""
    source = f"<p class=\"readme-source\">{html_escape(tx(locale, 'source', 'Source'))}: <code>{html_escape(page.isotope_readme_source)}</code></p>" if page.isotope_readme_source else ""
    return f"""
<div class="readme-excerpt">
  <p class="readme-label">{html_escape(tx(locale, 'localReadmeExcerpt', 'Local README excerpt'))}</p>
  {page.isotope_readme_html}
  {source}
</div>
"""


def render_gate(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    gate = page.approval_gate
    if not gate:
        return ""
    rules = "".join(f"<li>{html_escape(rule)}</li>" for rule in gate.get("rules", []))
    severities = ", ".join(gate.get("severities") or []) or tx(locale, "notSpecified", "not specified")
    entrypoints = ", ".join(gate.get("entrypoints") or []) or page.display_name
    coverage = gate.get("coverage_status") or tx(locale, "unknown", "unknown")
    reviewed = gate.get("reviewed_at") or ""
    return f"""
<section class="pkg-section split-section gate-section">
  <div>
    <p class="section-kicker">{html_escape(tx(locale, 'approvalGatesKicker', 'approval gates'))}</p>
    <h2>{html_escape(tx(locale, 'approvalGateHeading', 'Human review metadata for risky commands'))}</h2>
    <p>{html_escape(tx(locale, 'approvalGateCopy', 'The local approval-gate seed includes {count} rules for {name}. Covered entrypoints: {entrypoints}. Severity labels: {severities}. Coverage: {coverage}{reviewed}.', count=gate.get('rule_count'), name=page.display_name, entrypoints=entrypoints, severities=severities, coverage=coverage, reviewed=(f', {tx(locale, "reviewed", "reviewed")} {reviewed}' if reviewed else '')))}</p>
  </div>
  <div class="detail-stack">
    <article>
      <h3>{html_escape(tx(locale, 'exampleGatedActions', 'Example gated actions'))}</h3>
      <ul>{rules or f'<li>{html_escape(tx(locale, "noApprovalRules", "No rule descriptions were present."))}</li>'}</ul>
    </article>
  </div>
</section>
"""


def render_executables(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    executable_rows: list[str] = []
    seen: set[str] = set()
    for item in page.executables:
        name = str(item.get("name") or item.get("target") or item.get("source") or "").strip()
        if not name or name in seen:
            continue
        seen.add(name)
        executable_rows.append(executable_row(name, item.get("kind") or tx(locale, "executable", "executable"), item.get("exposure") or tx(locale, "globalExecutable", "global executable"), item.get("note") or ""))
    for item in page.binaries:
        if not isinstance(item, dict):
            continue
        name = str(item.get("target") or item.get("source") or "").strip()
        if name and name not in seen:
            seen.add(name)
            executable_rows.append(executable_row(name, tx(locale, "binary", "binary"), tx(locale, "homebrewCaskBinary", "Homebrew cask binary"), str(item.get("source") or "")))
    for alias in sorted(page.aliases):
        if alias not in seen:
            seen.add(alias)
            exposure = tx(locale, "stubExcluded", "Automic Vault stub excluded") if alias in page.extra.get("stub_exclusions", []) else tx(locale, "indexedExecutable", "indexed executable")
            executable_rows.append(executable_row(alias, tx(locale, "executable", "executable"), exposure, tx(locale, "localExecutableIndex", "Discovered from the local executable index.")))
    body = "".join(executable_rows)
    return f"""
<section class="pkg-section" aria-labelledby="executables-title">
  <p class="section-kicker">{html_escape(tx(locale, 'executables', 'executables'))}</p>
  <h2 id="executables-title">{html_escape(tx(locale, 'executablesTitle', 'Installed executables'))}</h2>
  <div class="table-wrap executable-table">
    <table>
      <thead><tr><th>{html_escape(tx(locale, 'command', 'Command'))}</th><th>{html_escape(tx(locale, 'kind', 'Kind'))}</th><th>{html_escape(tx(locale, 'exposure', 'Exposure'))}</th><th>{html_escape(tx(locale, 'note', 'Note'))}</th></tr></thead>
      <tbody>{body or f'<tr><td colspan="4">{html_escape(tx(locale, "executableDataMissing", "No executable data was present."))}</td></tr>'}</tbody>
    </table>
  </div>
</section>
"""


def freshness_item(item: dict[str, Any], locale: dict[str, Any] | None = None) -> str:
    severity = str(item.get("severity") or "info")
    kind = str(item.get("kind") or "freshness")
    message = str(item.get("message") or "").strip()
    evidence = str(item.get("evidence") or "").strip()
    confidence = str(item.get("confidence") or "").strip()
    evidence_html = link_value(evidence) if evidence else ""
    return f"""
<li class="freshness-item freshness-{attr(severity)}">
  <strong>{html_escape(severity)}</strong>
  <span>{html_escape(message or kind)}</span>
  {f'<small>{evidence_html}</small>' if evidence_html else ''}
  {f'<em>{html_escape(confidence)} {html_escape(tx(locale, "confidence", "confidence"))}</em>' if confidence else ''}
</li>
"""


def render_freshness(page: PackagePage, manifest: dict[str, Any], locale: dict[str, Any] | None = None) -> str:
    freshness = page.version_freshness or {}
    manager = freshness.get("packageManager") if isinstance(freshness.get("packageManager"), dict) else {}
    site = freshness.get("siteData") if isinstance(freshness.get("siteData"), dict) else {}
    upstream = freshness.get("upstream") if isinstance(freshness.get("upstream"), dict) else {}
    warnings = freshness.get("warnings") if isinstance(freshness.get("warnings"), list) else []
    version = manager.get("version") or page.version or tx(locale, "unknown", "unknown")
    manager_updated = fmt_date(str(manager.get("updatedAt") or page.last_updated_at or ""))
    site_status = site.get("status") or tx(locale, "unknown", "unknown")
    upstream_comparison = upstream.get("comparison") or tx(locale, "notAvailable", "not available")
    upstream_latest = upstream.get("latestVersion") or tx(locale, "notDetected", "not detected")
    repository = upstream.get("repository") or ""
    warning_items = "".join(freshness_item(item, locale) for item in warnings if isinstance(item, dict))
    if not warning_items:
        warning_items = f'<li class="freshness-item freshness-info"><strong>ok</strong><span>{html_escape(tx(locale, "noFreshnessWarnings", "No freshness warnings were generated."))}</span></li>'
    return f"""
<section class="pkg-section split-section freshness-section" aria-labelledby="freshness-title">
  <div>
    <p class="section-kicker">{html_escape(tx(locale, 'freshness', 'freshness'))}</p>
    <h2 id="freshness-title">{html_escape(tx(locale, 'freshnessTitle', 'Version and freshness'))}</h2>
    <p>{html_escape(tx(locale, 'freshnessCopy', 'These signals separate page generation age, package-manager activity, and upstream release comparison. Version lag is warned only when an evidence URL and comparable versions are present.'))}</p>
  </div>
  <div>
    <div class="freshness-metrics">
      <div><span>{html_escape(tx(locale, 'pageGenerated', 'page generated'))}</span><strong>{html_escape(fmt_date(manifest.get("generated_at", "")) or tx(locale, "unknown", "unknown"))}</strong></div>
      <div><span>{html_escape(tx(locale, 'managerVersion', 'manager version'))}</span><strong>{html_escape(version)}</strong></div>
      <div><span>{html_escape(tx(locale, 'managerUpdated', 'manager updated'))}</span><strong>{html_escape(manager_updated or tx(locale, "unknown", "unknown"))}</strong></div>
      <div><span>{html_escape(tx(locale, 'localData', 'local data'))}</span><strong>{html_escape(site_status)}</strong></div>
      <div><span>{html_escape(tx(locale, 'upstream', 'upstream'))}</span><strong>{html_escape(upstream_comparison)}</strong></div>
      <div><span>{html_escape(tx(locale, 'upstreamLatestDetected', 'latest detected'))}</span><strong>{html_escape(upstream_latest)}</strong></div>
    </div>
    {f'<p class="freshness-repo"><a href="{attr(repository)}">{html_escape(repository)}</a></p>' if repository else ''}
    <ul class="freshness-list">{warning_items}</ul>
  </div>
</section>
"""


def executable_row(name: str, kind: Any, exposure: Any, note: Any) -> str:
    return f"<tr><td><code>{html_escape(name)}</code></td><td>{html_escape(kind)}</td><td>{html_escape(exposure)}</td><td>{html_escape(note)}</td></tr>"


def render_install_metadata(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    rows: list[tuple[str, str]] = []
    for label, value in (
        (tx(locale, "packageKey", "Package key"), page.key),
        (tx(locale, "version", "Version"), page.version),
        (tx(locale, "packageManager", "Package manager"), package_manager_label(page)),
        (tx(locale, "packageManagerPage", "Package manager page"), page.package_manager_url),
        (tx(locale, "homepage", "Homepage"), page.homepage),
        (tx(locale, "repository", "Repository"), page.repository),
        (tx(locale, "upstreamDocs", "Upstream docs"), page.upstream_docs),
        (tx(locale, "license", "License"), page.license),
        (tx(locale, "sourceArchive", "Source archive"), page.source_archive),
        (tx(locale, "issueTracker", "Issue tracker"), page.issue_tracker),
        (tx(locale, "updated", "Last updated"), page.last_updated_at),
        (tx(locale, "verified", "Last verified"), page.last_verified),
        (tx(locale, "published", "Published"), page.published_at),
        ("Pulse", page.pulse_kind),
        ("SHA-256", page.sha256),
        (tx(locale, "downloadUrl", "Download URL"), page.url),
    ):
        if value:
            rows.append((label, value))
    if page.binaries:
        rows.append((tx(locale, "binaries", "Binaries"), ", ".join(sorted({item.get("target") or item.get("source") or "" for item in page.binaries if isinstance(item, dict)}))))
    if page.dependencies:
        rows.append((tx(locale, "dependencies", "Dependencies"), ", ".join(page.dependencies)))
    if page.build_dependencies:
        rows.append((tx(locale, "buildDependencies", "Build dependencies"), ", ".join(page.build_dependencies)))
    if page.uses_from_macos:
        rows.append((tx(locale, "usesFromMacos", "Uses from macOS"), ", ".join(page.uses_from_macos)))
    if page.bottle:
        bottle = tx(locale, "available", "available") if page.bottle.get("available") else tx(locale, "notRecorded", "not recorded")
        platforms = ", ".join(page.bottle.get("platforms") or [])
        rows.append((tx(locale, "bottle", "Bottle"), f"{bottle}{f' ({platforms})' if platforms else ''}"))
    if page.install_behavior:
        post_install = page.install_behavior.get("postInstallDefined")
        if post_install is not None:
            label = tx(locale, "npmPostinstall", "npm postinstall") if page.provider == "npm" else tx(locale, "homebrewPostinstall", "Homebrew post-install")
            rows.append((label, tx(locale, "defined", "defined") if post_install else tx(locale, "notDefined", "not defined")))
        service = page.install_behavior.get("service")
        rows.append((tx(locale, "service", "Service"), service if service else tx(locale, "serviceNone", "none declared")))
        caveats = page.install_behavior.get("caveats")
        if caveats:
            rows.append((tx(locale, "caveats", "Caveats"), caveats))
        lifecycle = page.install_behavior.get("lifecycleScripts")
        if lifecycle:
            rows.append((tx(locale, "lifecycleScripts", "npm lifecycle scripts"), ", ".join(str(item) for item in lifecycle)))
        python_requires = page.install_behavior.get("pythonRequires")
        if python_requires:
            rows.append((tx(locale, "pythonRequires", "Python requires"), str(python_requires)))
        requires_dist_count = page.install_behavior.get("requiresDistCount")
        if requires_dist_count:
            rows.append((tx(locale, "pypiDependencySpecs", "PyPI dependency specs"), fmt_int(requires_dist_count)))
    if page.keywords:
        rows.append((tx(locale, "keywords", "Keywords"), ", ".join(page.keywords[:16])))
    if page.classifiers:
        rows.append((tx(locale, "classifiers", "Classifiers"), ", ".join(page.classifiers[:12])))
    deps = page.extra.get("homebrewDeps") or page.extra.get("npm_homebrewDeps")
    if deps:
        rows.append((tx(locale, "homebrewDependencies", "Homebrew dependencies"), ", ".join(deps)))
    python_formula = page.extra.get("pythonFormula")
    if python_formula:
        rows.append((tx(locale, "pythonFormula", "Python formula"), python_formula))
    row_html = "".join(f"<tr><th>{html_escape(label)}</th><td>{link_value(value)}</td></tr>" for label, value in rows)
    return f"""
<section class="pkg-section">
  <p class="section-kicker">{html_escape(tx(locale, 'packageMetadataKicker', 'install metadata'))}</p>
  <h2>{html_escape(tx(locale, 'metadataTitle', 'Package metadata'))}</h2>
  <div class="table-wrap">
    <table>
      <tbody>{row_html or f'<tr><th>{html_escape(tx(locale, "status", "Status"))}</th><td>{html_escape(tx(locale, "metadataEmpty", "No resolver details were present."))}</td></tr>'}</tbody>
    </table>
  </div>
</section>
"""


def render_related(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    related = [related_link(item, locale) for item in page.related_packages]
    also = [related_link(item, locale) for item in page.also_available_via]
    hubs = [hub_link(item, locale) for item in page.package_hubs]
    if not related and not also:
        related = inferred_related_links(page, locale)
    return f"""
<section class="pkg-section split-section related-section" aria-labelledby="related-title">
  <div>
    <p class="section-kicker">{html_escape(tx(locale, 'packageGraph', 'package graph'))}</p>
    <h2 id="related-title">{html_escape(tx(locale, 'relatedPackages', 'Related packages'))}</h2>
    <p>{html_escape(tx(locale, 'packageGraphCopy', 'Links come from the local package relationship graph, supplements, dependency edges, ecosystem matches, and package hub membership.'))}</p>
  </div>
  <div class="related-columns">
    <article>
      <h3>{html_escape(tx(locale, 'related', 'Related'))}</h3>
      <ul>{''.join(related) or f'<li>{html_escape(tx(locale, "noRelated", "No related package links were present."))}</li>'}</ul>
    </article>
    <article>
      <h3>{html_escape(tx(locale, 'alsoAvailableVia', 'Also available via'))}</h3>
      <ul>{''.join(also) or f'<li>{html_escape(tx(locale, "noCrossEcosystem", "No cross-ecosystem equivalent was recorded."))}</li>'}</ul>
    </article>
    <article>
      <h3>{html_escape(tx(locale, 'packageHubs', 'Package hubs'))}</h3>
      <ul>{''.join(hubs) or f'<li>{html_escape(tx(locale, "noHubMembership", "No package hub membership was generated."))}</li>'}</ul>
    </article>
  </div>
</section>
"""


def inferred_related_links(page: PackagePage, locale: dict[str, Any] | None = None) -> list[str]:
    links: list[str] = []
    if page.provider != "brew":
        return links
    for dependency in page.dependencies[:6]:
        links.append(related_link({
            "provider": page.provider,
            "name": dependency,
            "label": dependency,
            "reason": f"{package_manager_label(page)} dependency.",
        }, locale))
    return links


def has_internal_package_navigation(page: PackagePage) -> bool:
    return bool(
        page.related_packages
        or page.also_available_via
        or page.package_hubs
        or inferred_related_links(page)
    )


def related_link(item: dict[str, Any], locale: dict[str, Any] | None = None) -> str:
    provider = str(item.get("provider") or "").strip()
    name = str(item.get("name") or "").strip()
    label = str(item.get("label") or name).strip()
    reason = str(item.get("reason") or "").strip()
    if not provider or not name:
        return ""
    href = f"../../{attr(provider)}/{attr(slugify(name))}/"
    return f'<li><a href="{href}">{html_escape(label)}</a>{f"<span>{html_escape(reason)}</span>" if reason else ""}</li>'


def hub_link(item: dict[str, Any], locale: dict[str, Any] | None = None) -> str:
    slug = str(item.get("slug") or "").strip()
    label = str(item.get("label") or slug).strip()
    reason = str(item.get("reason") or "").strip()
    if not slug:
        return ""
    return f'<li><a href="../../{attr(slug)}/">{html_escape(label)}</a>{f"<span>{html_escape(reason)}</span>" if reason else ""}</li>'


def link_value(value: str) -> str:
    if value.startswith("https://") or value.startswith("http://"):
        return f'<a href="{attr(value)}">{html_escape(value)}</a>'
    return html_escape(value)


def render_sources(page: PackagePage, locale: dict[str, Any] | None = None) -> str:
    notes = sorted(set(page.source_notes)) or [tx(locale, "localPackageGenerator", "local package generator")]
    note_html = "".join(f"<li>{html_escape(note)}</li>" for note in notes)
    return f"""
<section class="pkg-section split-section sources-section">
  <div>
    <p class="section-kicker">{html_escape(tx(locale, 'sourceTrail', 'source trail'))}</p>
    <h2>{html_escape(tx(locale, 'generatedFromRepositoryData', 'Generated from repository data'))}</h2>
    <p>{tx(locale, 'sourcesCopy', 'This page is written by <code>scripts/generate-pkg-pages.py</code>. Deployments refuse to publish if <code>www/pkg/</code> is stale relative to local package data.')}</p>
  </div>
  <div class="detail-stack">
    <article>
      <h3>{html_escape(tx(locale, 'usedSources', 'Used sources'))}</h3>
      <ul>{note_html}</ul>
    </article>
  </div>
</section>
"""


def schema_for_package(page: PackagePage, description: str, updated: str, locale: dict[str, Any] | None = None) -> dict[str, Any]:
    url = locale_url(page.path, locale)
    software: dict[str, Any] = {
        "@type": "SoftwareApplication",
        "@id": f"{url}#software",
        "name": page.display_name,
        "applicationCategory": "DeveloperApplication",
        "operatingSystem": "macOS",
        "url": url,
        "description": description,
        "dateModified": updated,
        "inLanguage": (locale or {}).get("htmlLang") or "en",
        "isPartOf": {"@id": f"{SITE_ORIGIN}/#website"},
    }
    if page.homepage:
        software["sameAs"] = page.homepage
    if page.version:
        software["softwareVersion"] = page.version
    if page.license:
        software["license"] = page.license
    if page.repository:
        software["codeRepository"] = page.repository
    if page.dependencies:
        software["softwareRequirements"] = ", ".join(page.dependencies[:16])

    article = {
        "@type": "TechArticle",
        "@id": f"{url}#article",
        "headline": tx(locale, "schemaTechArticleHeadline", "Install {name} with {manager}", name=page.display_name, manager=package_manager_label(page)),
        "description": description,
        "dateModified": updated,
        "inLanguage": (locale or {}).get("htmlLang") or "en",
        "author": {"@id": f"{SITE_ORIGIN}/about/#max-howell"},
        "reviewedBy": {"@id": f"{SITE_ORIGIN}/about/#max-howell"},
        "publisher": {"@id": f"{SITE_ORIGIN}/#organization"},
        "mainEntity": {"@id": f"{url}#software"},
    }
    breadcrumb = {
        "@type": "BreadcrumbList",
        "@id": f"{url}#breadcrumbs",
        "itemListElement": [
            {"@type": "ListItem", "position": 1, "name": tx(locale, "home", "Home"), "item": locale_url("/", locale)},
            {"@type": "ListItem", "position": 2, "name": tx(locale, "packages", "Packages"), "item": locale_url("/pkg/", locale)},
            {"@type": "ListItem", "position": 3, "name": page.display_name, "item": url},
        ],
    }
    how_to = {
        "@type": "HowTo",
        "@id": f"{url}#install-howto",
        "name": tx(locale, "schemaHowToName", "Install {name}", name=page.display_name),
        "step": [
            {
                "@type": "HowToStep",
                "position": index + 1,
                "name": tx(locale, "schemaHowToStep", "Run {manager} command", manager=item.get("manager") or tx(locale, "install", "install")),
                "text": str(item.get("command") or ""),
            }
            for index, item in enumerate(source_backed_schema_commands(page)[:12])
            if str(item.get("command") or "").strip()
        ],
    }
    return {
        "@context": "https://schema.org",
        "@graph": [
            {"@type": "WebSite", "@id": f"{SITE_ORIGIN}/#website", "name": "Automic Vault", "url": f"{SITE_ORIGIN}/"},
            {"@type": "Organization", "@id": f"{SITE_ORIGIN}/#organization", "name": "Automic Vault", "url": f"{SITE_ORIGIN}/"},
            {"@type": "Person", "@id": f"{SITE_ORIGIN}/about/#max-howell", "name": "Max Howell", "url": f"{SITE_ORIGIN}/about/"},
            software,
            article,
            breadcrumb,
            how_to,
        ],
    }


def copy_script(locale: dict[str, Any] | None = None) -> str:
    copied = json.dumps(tx(locale, "copied", "Copied"), ensure_ascii=False)
    failed = json.dumps(tx(locale, "copyFailed", "Copy failed"), ensure_ascii=False)
    return f"""  <script>
    document.addEventListener("click", async (event) => {{
      const button = event.target.closest("[data-copy]");
      if (!button) return;
      try {{
        await navigator.clipboard.writeText(button.getAttribute("data-copy"));
        const previous = button.textContent;
        button.textContent = {copied};
        button.setAttribute("data-state", "copied");
        window.setTimeout(() => {{
          button.textContent = previous;
          button.removeAttribute("data-state");
        }}, 1600);
      }} catch (_error) {{
        button.textContent = {failed};
        button.setAttribute("data-state", "error");
      }}
    }});
  </script>"""


def render_sitemap_index(sitemap_names: list[str], manifest: dict[str, Any]) -> str:
    lastmod = fmt_date(manifest.get("generated_at", ""))
    entries = "\n".join(
        f"  <sitemap>\n    <loc>{SITE_ORIGIN}/pkg/{name}</loc>\n    <lastmod>{lastmod}</lastmod>\n  </sitemap>"
        for name in sitemap_names
    )
    return '<?xml version="1.0" encoding="UTF-8"?>\n<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n' + entries + "\n</sitemapindex>\n"


def render_hub_sitemap(hubs: list[tuple[PackageHub, list[PackagePage]]], manifest: dict[str, Any]) -> str:
    lastmod = fmt_date(manifest.get("generated_at", ""))
    urls = [sitemap_url(f"{SITE_ORIGIN}/pkg/", lastmod, "/pkg/")]
    urls.extend(sitemap_url(f"{SITE_ORIGIN}{hub.path}", lastmod, hub.path) for hub, _hub_pages in hubs)
    return render_urlset(urls)


def render_package_sitemap(pages: list[PackagePage], manifest: dict[str, Any]) -> str:
    lastmod = fmt_date(manifest.get("generated_at", ""))
    urls = [
        sitemap_url(f"{SITE_ORIGIN}{page.path}", fmt_date(page.last_updated_at) or lastmod, page.path)
        for page in pages
    ]
    return render_urlset(urls)


def sitemap_url(loc: str, lastmod: str, path: str | None = None) -> str:
    lines = ["  <url>", f"    <loc>{loc}</loc>", f"    <lastmod>{lastmod}</lastmod>"]
    if path:
        lines.extend(sitemap_hreflang_lines(path))
    lines.append("  </url>")
    return "\n".join(lines)


def render_urlset(urls: list[str]) -> str:
    return '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n' + "\n".join(urls) + "\n</urlset>\n"


def sitemap_hreflang_lines(path: str) -> list[str]:
    lines = [f'    <xhtml:link rel="alternate" hreflang="en" href="{SITE_ORIGIN}{path}" />']
    for locale in non_default_i18n_locales():
        lines.append(
            f'    <xhtml:link rel="alternate" hreflang="{locale.get("hreflang")}" href="{locale_url(path, locale)}" />'
        )
    lines.append(f'    <xhtml:link rel="alternate" hreflang="x-default" href="{SITE_ORIGIN}{path}" />')
    return lines


def nav(root: str, locale: dict[str, Any] | None = None) -> str:
    return f"""
<header class="masthead">
  <a class="brand" href="{root}" aria-label="Automic Vault home">
    <img class="brand-mark" src="{root}assets/icon@2x.webp" alt="" width="54" height="54">
    <span class="brand-type">Automic Vault</span>
  </a>
  <nav class="nav" aria-label="Main navigation">
    <a href="{root}docs/">{html_escape(tx(locale, 'docs', 'Docs'))}</a>
    <a href="{root}security/">{html_escape(tx(locale, 'security', 'Security'))}</a>
    <a href="{root}pkg/">{html_escape(tx(locale, 'packages', 'Packages'))}</a>
    <a href="https://github.com/automic-vault/">{html_escape(tx(locale, 'github', 'GitHub'))}</a>
  </nav>
</header>
"""


def footer(root: str, locale: dict[str, Any] | None = None) -> str:
    return f"""
<footer class="site-footer">
  <p>{html_escape(tx(locale, 'footer', 'Automic Vault protects AI agent runs with local secret storage, approval gates, and hardened package installs.'))}</p>
  <div class="footer-links">
    <a href="{root}privacy/">{html_escape(tx(locale, 'privacy', 'Privacy'))}</a>
    <a href="{root}terms/">{html_escape(tx(locale, 'terms', 'Terms'))}</a>
    <a href="{root}llms.txt">llms.txt</a>
  </div>
</footer>
"""


def html_hreflang_links(path: str) -> str:
    lines = [f'  <link rel="alternate" hreflang="en" href="{SITE_ORIGIN}{path}">']
    for locale in non_default_i18n_locales():
        lines.append(f'  <link rel="alternate" hreflang="{locale.get("hreflang")}" href="{locale_url(path, locale)}">')
    lines.append(f'  <link rel="alternate" hreflang="x-default" href="{SITE_ORIGIN}{path}">')
    return "\n".join(lines)


def replace_schema_url(value: Any, source_url: str, target_url: str) -> Any:
    if isinstance(value, str):
        return value.replace(source_url, target_url)
    if isinstance(value, list):
        return [replace_schema_url(item, source_url, target_url) for item in value]
    if isinstance(value, dict):
        return {key: replace_schema_url(item, source_url, target_url) for key, item in value.items()}
    return value


def html_doc(
    title: str,
    description: str,
    canonical: str,
    body: str,
    stylesheet_href: str,
    favicon_href: str,
    schema: dict[str, Any],
    robots: str = "index,follow",
    extra_head: str = "",
    extra_body: str = "",
    alternates_path: str | None = None,
    locale: dict[str, Any] | None = None,
) -> str:
    if alternates_path:
        schema = replace_schema_url(copy.deepcopy(schema), f"{SITE_ORIGIN}{alternates_path}", canonical)
    schema_json = json.dumps(schema, indent=2, ensure_ascii=False)
    hreflang_head = html_hreflang_links(alternates_path) if alternates_path else ""
    html_lang = str((locale or {}).get("htmlLang") or "en")
    return f"""<!DOCTYPE html>
<html lang="{attr(html_lang)}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{html_escape(title)}</title>
  <meta name="description" content="{attr(description)}">
  <meta name="robots" content="{attr(robots)}">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="Automic Vault">
  <meta property="og:title" content="{attr(title)}">
  <meta property="og:description" content="{attr(description)}">
  <meta property="og:url" content="{attr(canonical)}">
  <meta property="og:image" content="{SITE_ORIGIN}/preview.jpg">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="{attr(title)}">
  <meta name="twitter:description" content="{attr(description)}">
  <meta name="twitter:image" content="{SITE_ORIGIN}/preview.jpg">
  <link rel="canonical" href="{attr(canonical)}">
{hreflang_head}
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600;700;800&amp;family=Geist+Mono:wght@400;500;600;700&amp;display=swap" rel="stylesheet">
  <link rel="icon" href="{favicon_href}" sizes="16x16 32x32 48x48">
  <link rel="stylesheet" href="{stylesheet_href}">
{GOOGLE_TAG}
{extra_head}
  <script type="application/ld+json">
{schema_json}
  </script>
</head>
<body>
  <div class="site-shell">
{textwrap.indent(body.strip(), '    ')}
  </div>
{extra_body}
</body>
</html>
"""


def render_css() -> str:
    return """:root {
  --bg: #10100f;
  --surface: #171615;
  --surface-2: #1d1c1a;
  --ink: #f0eee8;
  --muted: #9e9a90;
  --dim: #6f6a62;
  --line: #302e2b;
  --line-strong: #45413b;
  --hot: #f26d3d;
  --blue: #2d8bd8;
  --green: #72b661;
  --gold: #d0a248;
  --font-ui: "Geist", system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --font-mono: "Geist Mono", "SFMono-Regular", Consolas, monospace;
  --max: 1540px;
}

* { box-sizing: border-box; }

html {
  background: #050505;
  color: var(--ink);
  scroll-behavior: smooth;
}

body {
  margin: 0;
  min-height: 100vh;
  background:
    linear-gradient(180deg, rgba(34, 33, 30, 0.92), rgba(8, 8, 8, 0.98) 42rem),
    #080808;
  font-family: var(--font-ui);
  letter-spacing: 0;
}

body::before {
  content: "";
  position: fixed;
  inset: 0;
  z-index: 50;
  pointer-events: none;
  background:
    linear-gradient(rgba(255, 255, 255, 0.025) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255, 255, 255, 0.018) 1px, transparent 1px);
  background-size: 100% 4px, 48px 100%;
  opacity: 0.22;
  mix-blend-mode: screen;
}

a { color: inherit; text-decoration: none; }
a:focus-visible { outline: 1px solid var(--gold); outline-offset: 4px; }
h1, h2, h3, h4, p, ul, ol, pre { margin: 0; }
code { font-family: var(--font-mono); }

.site-shell {
  width: min(calc(100% - 48px), var(--max));
  margin: 44px auto;
  overflow: clip;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: rgba(19, 18, 17, 0.96);
  box-shadow: 0 34px 90px rgba(0, 0, 0, 0.44);
}

.masthead {
  position: sticky;
  top: 0;
  z-index: 40;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  min-height: 82px;
  padding: 18px clamp(20px, 3vw, 44px);
  border-bottom: 1px solid var(--line);
  background: rgba(23, 22, 21, 0.9);
  backdrop-filter: blur(16px);
}

.brand { display: inline-flex; align-items: center; gap: 12px; min-width: 0; }
.brand-mark { width: 34px; height: 34px; border-radius: 8px; }
.brand-type {
  color: var(--ink);
  font-family: var(--font-mono);
  font-size: 0.96rem;
  font-weight: 700;
  text-transform: uppercase;
}

.nav {
  display: flex;
  align-items: center;
  gap: clamp(14px, 2.4vw, 32px);
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: 0.79rem;
  font-weight: 600;
  text-transform: uppercase;
}
.nav a { padding: 8px 0; transition: color 160ms ease, transform 160ms ease; }
.nav a:hover { color: var(--ink); transform: translateY(-1px); }

.breadcrumbs {
  display: flex;
  flex-wrap: wrap;
  gap: 9px;
  padding: 22px clamp(20px, 3vw, 44px) 0;
  color: var(--dim);
  font-family: var(--font-mono);
  font-size: 0.76rem;
  font-weight: 700;
  text-transform: uppercase;
}
.breadcrumbs a:hover { color: var(--ink); }

.pkg-hero {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(310px, 0.36fr);
  gap: clamp(34px, 6vw, 84px);
  align-items: end;
  padding: clamp(42px, 6vw, 88px) clamp(20px, 3vw, 44px) clamp(32px, 5vw, 68px);
  border-bottom: 1px solid var(--line);
}
.pkg-hero-index { align-items: center; }
.hero-copy, .hero-panel { min-width: 0; }
.eyebrow, .section-kicker {
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: 0.79rem;
  font-weight: 700;
  line-height: 1.45;
  text-transform: uppercase;
}
h1 {
  max-width: 14ch;
  margin-top: 14px;
  color: var(--ink);
  font-size: clamp(3.2rem, 7.6vw, 7.8rem);
  font-weight: 800;
  line-height: 0.88;
  overflow-wrap: anywhere;
  text-transform: uppercase;
}
.lede {
  width: min(100%, 820px);
  margin-top: clamp(20px, 3vw, 34px);
  color: var(--ink);
  font-size: clamp(1.45rem, 2.4vw, 2.15rem);
  font-weight: 600;
  line-height: 1.12;
}
.hero-actions { display: flex; flex-wrap: wrap; gap: 12px; margin-top: clamp(28px, 4vw, 48px); }
.button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 46px;
  padding: 12px 18px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  color: var(--ink);
  font-family: var(--font-mono);
  font-size: 0.83rem;
  font-weight: 700;
  text-transform: uppercase;
  transition: border-color 160ms ease, background 160ms ease, color 160ms ease, transform 160ms ease;
}
.button:hover { transform: translateY(-1px); }
.button:active { transform: translateY(1px); }
.button.primary { border-color: var(--hot); background: var(--hot); color: #11100f; }
.button.secondary { background: rgba(255, 255, 255, 0.035); }
.copy-button {
  min-height: 36px;
  padding: 8px 12px;
  border: 1px solid var(--line-strong);
  border-radius: 7px;
  background: rgba(255, 255, 255, 0.04);
  color: var(--ink);
  cursor: pointer;
  font-family: var(--font-mono);
  font-size: 0.74rem;
  font-weight: 700;
  text-transform: uppercase;
  transition: transform 180ms cubic-bezier(0.16, 1, 0.3, 1), border-color 180ms cubic-bezier(0.16, 1, 0.3, 1), background 180ms cubic-bezier(0.16, 1, 0.3, 1);
}
.copy-button:hover { border-color: var(--hot); }
.copy-button:active { transform: translateY(1px) scale(0.98); }
.copy-button[data-state="copied"] { border-color: rgba(114, 182, 97, 0.72); color: var(--green); }
.copy-button[data-state="error"] { border-color: rgba(242, 109, 61, 0.72); color: var(--hot); }
.terminal-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}
.download-av-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 36px;
  padding: 8px 12px;
  border: 1px solid rgba(242, 109, 61, 0.72);
  border-radius: 7px;
  background: rgba(242, 109, 61, 0.12);
  color: var(--ink);
  cursor: pointer;
  font-family: var(--font-mono);
  font-size: 0.74rem;
  font-weight: 700;
  text-transform: uppercase;
  transition: transform 180ms cubic-bezier(0.16, 1, 0.3, 1), border-color 180ms cubic-bezier(0.16, 1, 0.3, 1), background 180ms cubic-bezier(0.16, 1, 0.3, 1);
}
.download-av-button:hover {
  border-color: var(--hot);
  background: rgba(242, 109, 61, 0.18);
}
.download-av-button:active { transform: translateY(1px) scale(0.98); }

.hero-panel {
  display: grid;
  border-top: 1px solid var(--line-strong);
}
.metric {
  display: grid;
  grid-template-columns: minmax(0, 0.45fr) minmax(0, 1fr);
  gap: 14px;
  padding: 16px 0;
  border-bottom: 1px solid var(--line);
}
.metric span {
  color: var(--dim);
  font-family: var(--font-mono);
  font-size: 0.74rem;
  font-weight: 700;
  text-transform: uppercase;
}
.metric strong {
  min-width: 0;
  color: var(--ink);
  font-size: 1rem;
  line-height: 1.2;
  overflow-wrap: anywhere;
}

.pkg-section {
  padding: clamp(30px, 4.5vw, 58px) clamp(20px, 3vw, 44px);
  border-bottom: 1px solid var(--line);
}
.split-section {
  display: grid;
  grid-template-columns: minmax(0, 0.8fr) minmax(300px, 1fr);
  gap: clamp(30px, 5vw, 70px);
  align-items: start;
}
.security-section {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(300px, 0.42fr);
  gap: clamp(30px, 5vw, 70px);
  background: linear-gradient(90deg, rgba(242, 109, 61, 0.055), transparent 34%);
}
.gate-section { background: rgba(45, 139, 216, 0.035); }
.sources-section { background: rgba(255, 255, 255, 0.018); }
.pkg-search-section {
  display: grid;
  grid-template-columns: minmax(220px, 0.34fr) minmax(0, 1fr);
  gap: clamp(22px, 4vw, 58px);
  align-items: center;
  padding-top: clamp(22px, 3.2vw, 38px);
  padding-bottom: clamp(22px, 3.2vw, 38px);
  background:
    linear-gradient(90deg, rgba(114, 182, 97, 0.052), transparent 44%),
    rgba(255, 255, 255, 0.012);
}
.search-copy p {
  max-width: 520px;
  margin-top: 10px;
  font-size: 0.96rem;
  line-height: 1.45;
}
.pkg-section h2 {
  max-width: 780px;
  margin-top: 8px;
  color: var(--ink);
  font-size: clamp(2rem, 3.4vw, 4rem);
  line-height: 0.95;
  text-transform: uppercase;
}
.pkg-section p {
  max-width: 820px;
  margin-top: 18px;
  color: var(--muted);
  font-size: 1.05rem;
  line-height: 1.58;
}
.install-section {
  display: grid;
  grid-template-columns: minmax(0, 1.08fr) minmax(300px, 0.72fr);
  gap: clamp(24px, 4.5vw, 64px);
  align-items: start;
  background: rgba(255, 255, 255, 0.014);
}
.install-command-panel {
  display: grid;
  gap: 22px;
}
.terminal-block {
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: #10100f;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);
}
.terminal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: 52px;
  padding: 8px 10px 8px 16px;
  border-bottom: 1px solid var(--line);
  color: var(--dim);
  font-family: var(--font-mono);
  font-size: 0.74rem;
  font-weight: 700;
  text-transform: uppercase;
}
.terminal-block pre {
  overflow-x: auto;
  padding: 24px;
  color: var(--ink);
  font-family: var(--font-mono);
  font-size: clamp(1rem, 2vw, 1.25rem);
  line-height: 1.5;
}
.platform-install-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}
.platform-install-grid article {
  min-width: 0;
  padding: 16px;
  border: 1px solid var(--line-strong);
  background: var(--surface-2);
}
.platform-install-grid h3 {
  color: var(--ink);
  font-size: 0.92rem;
  line-height: 1.2;
  text-transform: uppercase;
}
.install-command-list {
  display: grid;
  gap: 12px;
  margin-top: 14px;
}
.install-command-row {
  display: block;
  min-width: 0;
  padding-top: 14px;
  border-top: 1px solid var(--line);
}
.install-command-row:first-child { border-top: 0; padding-top: 0; }
.install-command-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  min-width: 0;
  margin-bottom: 6px;
}
.install-command-eyebrow {
  color: var(--ink);
  font-size: 0.72rem;
  font-weight: 800;
  letter-spacing: 0.04em;
  line-height: 1.1;
  text-transform: uppercase;
}
.install-command-head span {
  flex: 0 0 auto;
  color: var(--dim);
  font-family: var(--font-mono);
  font-size: 0.68rem;
  font-weight: 700;
  text-transform: uppercase;
}
.install-command-shell {
  position: relative;
  min-width: 0;
  min-height: 48px;
  padding: 13px 88px 13px 14px;
  border: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.026);
}
.install-command-shell code {
  display: block;
  min-width: 0;
  overflow-wrap: anywhere;
  color: var(--ink);
  font-family: var(--font-mono);
  font-size: 0.9rem;
  line-height: 1.45;
}
.install-command-shell .copy-button {
  position: absolute;
  top: 8px;
  right: 8px;
  min-height: 32px;
}
.install-command-source {
  margin: 8px 0 0;
  color: var(--dim);
  font-size: 0.82rem;
  line-height: 1.35;
}
.install-command-source a { color: var(--muted); }
.install-command-source a:hover { color: var(--ink); }
.install-notes-grid,
.signal-grid,
.related-columns {
  display: grid;
  gap: 12px;
}
.install-notes-grid article,
.signal-grid article,
.related-columns article {
  padding: 18px;
  border-top: 1px solid var(--line-strong);
  background: rgba(255, 255, 255, 0.018);
}
.install-notes-grid h3,
.signal-grid h3,
.related-columns h3 {
  color: var(--ink);
  font-size: 1.02rem;
  line-height: 1.2;
}
.install-notes-grid p,
.install-notes-grid ul,
.signal-grid p,
.signal-grid ul,
.related-columns p,
.related-columns ul {
  margin-top: 10px;
  color: var(--muted);
  line-height: 1.5;
}
.install-notes-grid ul,
.signal-grid ul,
.related-columns ul {
  padding-left: 1.1rem;
}
.install-notes-grid li + li,
.signal-grid li + li,
.related-columns li + li {
  margin-top: 8px;
}
.signal-grid {
  grid-template-columns: repeat(3, minmax(0, 1fr));
  margin-top: 28px;
}
.signal-grid strong {
  color: var(--ink);
  font-family: var(--font-mono);
}
.related-columns {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}
.related-section {
  grid-template-columns: minmax(260px, 0.35fr) minmax(0, 1fr);
}
.related-columns a {
  color: var(--ink);
  text-decoration: underline;
  text-decoration-color: var(--hot);
  text-underline-offset: 0.22em;
}
.related-columns span {
  display: block;
  margin-top: 4px;
  color: var(--muted);
  font-size: 0.92rem;
}
.executable-table td:first-child {
  color: var(--ink);
  font-family: var(--font-mono);
  font-weight: 700;
}
.freshness-section {
  grid-template-columns: minmax(260px, 0.35fr) minmax(0, 1fr);
}
.freshness-metrics {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1px;
  border: 1px solid var(--line-strong);
  background: var(--line-strong);
}
.freshness-metrics div {
  min-width: 0;
  padding: 14px;
  background: var(--surface-2);
}
.freshness-metrics span {
  display: block;
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
}
.freshness-metrics strong {
  display: block;
  margin-top: 8px;
  color: var(--ink);
  font-family: var(--font-mono);
  overflow-wrap: anywhere;
}
.freshness-repo {
  margin-top: 14px;
  color: var(--muted);
  overflow-wrap: anywhere;
}
.freshness-repo a {
  color: var(--ink);
  text-decoration: underline;
  text-decoration-color: var(--hot);
  text-underline-offset: 0.22em;
}
.freshness-list {
  display: grid;
  gap: 10px;
  margin-top: 16px;
  padding: 0;
  list-style: none;
}
.freshness-item {
  display: grid;
  grid-template-columns: minmax(72px, max-content) minmax(0, 1fr);
  gap: 6px 12px;
  padding: 14px;
  border: 1px solid var(--line-strong);
  background: var(--surface-2);
}
.freshness-item strong {
  color: var(--ink);
  font-family: var(--font-mono);
  font-size: 0.76rem;
  text-transform: uppercase;
}
.freshness-item span {
  color: var(--muted);
  line-height: 1.45;
}
.freshness-item small,
.freshness-item em {
  grid-column: 2;
  color: var(--muted);
  font-size: 0.86rem;
  font-style: normal;
  overflow-wrap: anywhere;
}
.freshness-item small a {
  color: var(--ink);
  text-decoration: underline;
  text-decoration-color: var(--hot);
  text-underline-offset: 0.22em;
}
.freshness-warning {
  border-color: rgba(242, 178, 61, 0.58);
  background: rgba(242, 178, 61, 0.08);
}
.freshness-notice {
  border-color: rgba(242, 178, 61, 0.38);
}
.detail-stack { display: grid; gap: 12px; }
.detail-stack article {
  padding: 18px;
  border: 1px solid var(--line-strong);
  background: var(--surface-2);
}
.detail-stack h3 {
  color: var(--ink);
  font-size: 1.08rem;
  line-height: 1.2;
}
.detail-stack p, .detail-stack ul { margin-top: 10px; color: var(--muted); line-height: 1.5; }
.detail-stack ul { padding-left: 1.1rem; }
.detail-stack li + li { margin-top: 8px; }
.detail-stack a, table a { color: var(--ink); text-decoration: underline; text-decoration-color: var(--hot); text-underline-offset: 0.22em; overflow-wrap: anywhere; }
.pkg-search {
  --pagefind-ui-primary: var(--ink);
  --pagefind-ui-text: var(--ink);
  --pagefind-ui-background: transparent;
  --pagefind-ui-border: var(--line-strong);
  --pagefind-ui-tag: var(--surface-2);
  --pagefind-ui-border-width: 1px;
  --pagefind-ui-border-radius: 8px;
  --pagefind-ui-image-border-radius: 6px;
  --pagefind-ui-font: var(--font-ui);
  min-width: 0;
}
.pkg-search .pagefind-ui__form {
  margin: 0;
}
.pkg-search .pagefind-ui__form::before { display: none; }
.pkg-search .pagefind-ui__search-input {
  height: 54px;
  padding: 0 18px 0 20px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface-2);
  color: var(--ink);
  font-family: var(--font-mono);
  font-size: clamp(0.82rem, 1.5vw, 0.95rem);
  font-weight: 700;
  transition: border-color 160ms ease, background 160ms ease, transform 160ms ease;
}
.pkg-search .pagefind-ui__search-input:focus {
  border-color: rgba(114, 182, 97, 0.72);
  background: #22211f;
  outline: none;
  transform: translateY(-1px);
}
.pkg-search .pagefind-ui__search-input::placeholder {
  color: #8d887f;
  opacity: 1;
}
.pkg-search .pagefind-ui__drawer {
  margin-top: 14px;
}
.pkg-search .pagefind-ui__message,
.pkg-search .pagefind-ui__result-excerpt,
.pkg-search .pagefind-ui__result-nested {
  color: var(--muted);
}
.pkg-search .pagefind-ui__result {
  padding: 18px 0;
  border-top: 1px solid var(--line);
}
.pkg-search .pagefind-ui__result-title {
  color: var(--ink);
  font-size: 1.08rem;
  line-height: 1.25;
}
.pkg-search .pagefind-ui__result-title a {
  text-decoration: underline;
  text-decoration-color: var(--hot);
  text-underline-offset: 0.22em;
}
.pkg-search mark {
  background: rgba(242, 109, 61, 0.18);
  color: var(--ink);
}
.readme-excerpt {
  max-width: 860px;
  margin-top: 28px;
  padding-top: 22px;
  border-top: 1px solid var(--line-strong);
}
.readme-excerpt h3 {
  color: var(--ink);
  font-size: clamp(1.45rem, 2.2vw, 2.35rem);
  line-height: 1.05;
  text-transform: uppercase;
}
.readme-excerpt h4 {
  margin-top: 24px;
  color: var(--ink);
  font-size: 1.15rem;
  line-height: 1.25;
}
.readme-excerpt p,
.readme-excerpt ul,
.readme-excerpt ol,
.readme-excerpt pre {
  margin-top: 14px;
}
.readme-excerpt ul,
.readme-excerpt ol {
  padding-left: 1.25rem;
  color: var(--muted);
  line-height: 1.55;
}
.readme-excerpt li + li { margin-top: 8px; }
.readme-excerpt code {
  color: var(--ink);
  font-size: 0.92em;
}
.readme-excerpt pre {
  overflow-x: auto;
  padding: 14px;
  border: 1px solid var(--line);
  background: rgba(0, 0, 0, 0.24);
}
.readme-excerpt a {
  color: var(--ink);
  text-decoration: underline;
  text-decoration-color: var(--hot);
  text-underline-offset: 0.22em;
}
.readme-label,
.readme-source {
  color: var(--dim);
  font-family: var(--font-mono);
  font-size: 0.74rem;
  font-weight: 700;
  text-transform: uppercase;
}
.chip-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 0;
  list-style: none;
}
.chip-list li {
  padding: 5px 8px;
  border: 1px solid var(--line-strong);
  border-radius: 999px;
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: 0.76rem;
  font-weight: 700;
}
.table-wrap { margin-top: 22px; overflow-x: auto; border-top: 1px solid var(--line-strong); }
table { width: 100%; border-collapse: collapse; }
th, td { padding: 16px 12px; border-bottom: 1px solid var(--line); text-align: left; vertical-align: top; }
th {
  width: 230px;
  color: var(--dim);
  font-family: var(--font-mono);
  font-size: 0.76rem;
  text-transform: uppercase;
}
td { color: var(--ink); overflow-wrap: anywhere; }
.package-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1px;
  background: var(--line);
  border: 1px solid var(--line);
}
.package-row {
  display: grid;
  gap: 8px;
  min-height: 82px;
  padding: 14px;
  background: var(--surface-2);
  transition: background 160ms ease, transform 160ms ease;
}
.package-row:hover { background: #22211f; transform: translateY(-1px); }
.package-row span { color: var(--ink); font-weight: 700; overflow-wrap: anywhere; }
.package-row small {
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
}
.hub-grid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 1px;
  margin-top: 24px;
  border: 1px solid var(--line);
  background: var(--line);
}
.hub-card {
  display: grid;
  min-height: 160px;
  gap: 12px;
  align-content: space-between;
  padding: 18px;
  background: var(--surface-2);
  transition: background 160ms ease, transform 160ms ease;
}
.hub-card:hover { background: #22211f; transform: translateY(-1px); }
.hub-card span {
  color: var(--ink);
  font-size: 1.05rem;
  font-weight: 800;
  line-height: 1.05;
  text-transform: uppercase;
  overflow-wrap: anywhere;
}
.hub-card strong {
  color: var(--hot);
  font-family: var(--font-mono);
  font-size: 2rem;
  line-height: 1;
}
.hub-card small {
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
}
.hub-table td:first-child { min-width: 160px; font-weight: 700; }
.pkg-concept-install {
  background:
    linear-gradient(90deg, rgba(114, 182, 97, 0.058), transparent 38%),
    rgba(255, 255, 255, 0.014);
}
.pkg-concept-section-head {
  display: grid;
  grid-template-columns: minmax(250px, 0.36fr) minmax(0, 0.72fr);
  gap: clamp(18px, 4vw, 54px);
  align-items: start;
}
.pkg-concept-section-head .section-kicker {
  padding-top: 10px;
}
.pkg-concept-section-head h2 {
  max-width: 840px;
  margin-top: 0;
  font-size: clamp(2.2rem, 4.2vw, 5.4rem);
  line-height: 0.92;
}
.pkg-concept-section-head p:not(.section-kicker) {
  grid-column: 2;
  max-width: 760px;
  margin-top: -22px;
}
.pkg-concept-primary-command {
  position: relative;
  min-width: 0;
  overflow: hidden;
  margin-top: clamp(28px, 4vw, 52px);
  border: 1px solid rgba(114, 182, 97, 0.44);
  border-radius: 9px;
  background:
    linear-gradient(135deg, rgba(114, 182, 97, 0.12), transparent 48%),
    #11110f;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.08);
}
.pkg-concept-primary-command::after {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: linear-gradient(180deg, transparent, rgba(114, 182, 97, 0.1), transparent);
  opacity: 0.52;
  transform: translateY(-100%);
  animation: pkg-scanline 6.5s cubic-bezier(0.16, 1, 0.3, 1) infinite;
}
.pkg-concept-primary-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: 54px;
  padding: 10px 10px 10px 16px;
  border-bottom: 1px solid rgba(114, 182, 97, 0.24);
  color: var(--green);
  font-family: var(--font-mono);
  font-size: 0.74rem;
  font-weight: 800;
  text-transform: uppercase;
}
.pkg-concept-primary-command pre {
  min-width: 0;
  overflow-x: auto;
  padding: clamp(22px, 4vw, 38px);
  color: var(--ink);
  font-family: var(--font-mono);
  font-size: clamp(1.16rem, 2.4vw, 2rem);
  line-height: 1.35;
}
.pkg-concept-platform-grid {
  display: grid;
  grid-template-columns: minmax(0, 0.85fr) minmax(0, 1.15fr);
  gap: 1px;
  margin-top: clamp(28px, 4vw, 52px);
  border: 1px solid var(--line);
  background: var(--line);
}
.pkg-concept-platform {
  min-width: 0;
  padding: clamp(16px, 2vw, 24px);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.026), transparent),
    var(--surface-2);
  animation: pkg-rise 520ms cubic-bezier(0.16, 1, 0.3, 1) both;
  animation-delay: calc(var(--i) * 90ms);
}
.pkg-concept-platform:nth-child(2) {
  grid-row: span 2;
}
.pkg-concept-platform-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 14px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--line);
}
.pkg-concept-platform h3 {
  color: var(--ink);
  font-size: clamp(1.12rem, 1.6vw, 1.55rem);
  line-height: 1;
  text-transform: uppercase;
}
.pkg-concept-platform-head span {
  color: var(--dim);
  font-family: var(--font-mono);
  font-size: 0.72rem;
  font-weight: 800;
  text-transform: uppercase;
}
.pkg-concept-command-list {
  display: grid;
}
.pkg-concept-command-row {
  display: block;
  padding: 16px 0;
  border-bottom: 1px solid var(--line);
}
.pkg-concept-command-row:last-child {
  border-bottom: 0;
}
.pkg-concept-command-row .install-command-shell {
  background: rgba(255, 255, 255, 0.032);
}
.pkg-concept-command-row .install-command-source {
  max-width: 58rem;
}
@keyframes pkg-scanline {
  0%, 38% { transform: translateY(-100%); }
  62%, 100% { transform: translateY(100%); }
}
@keyframes pkg-rise {
  from { opacity: 0; transform: translateY(14px); }
  to { opacity: 1; transform: translateY(0); }
}
.site-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 22px;
  padding: 24px clamp(20px, 3vw, 44px);
  color: var(--dim);
  font-size: 0.9rem;
}
.site-footer p { max-width: 620px; }
.footer-links { display: flex; flex-wrap: wrap; gap: 16px; font-family: var(--font-mono); font-size: 0.74rem; font-weight: 700; text-transform: uppercase; }
.footer-links a:hover { color: var(--ink); }

@media (max-width: 860px) {
  .site-shell { width: min(calc(100% - 24px), var(--max)); margin: 12px auto; }
  .masthead, .site-footer { align-items: flex-start; flex-direction: column; }
  .nav { width: 100%; flex-wrap: wrap; gap: 12px 18px; }
  .pkg-hero, .split-section, .security-section, .pkg-search-section, .install-section, .signal-grid, .related-columns, .platform-install-grid, .install-command-row, .freshness-metrics { grid-template-columns: 1fr; }
  .pkg-concept-section-head,
  .pkg-concept-platform-grid,
  .pkg-concept-command-row {
    grid-template-columns: 1fr;
  }
  .pkg-concept-section-head p:not(.section-kicker) { grid-column: auto; margin-top: 0; }
  .pkg-concept-platform:nth-child(2) { grid-row: auto; }
  .pkg-concept-command-row p { grid-column: 1 / -1; }
  .pkg-hero { padding-top: 38px; }
  .terminal-head {
    align-items: flex-start;
    flex-direction: column;
  }
  .terminal-actions { justify-content: flex-start; }
  h1 { font-size: clamp(2.8rem, 15vw, 4.8rem); }
  .lede { font-size: 1.32rem; }
  .package-list { grid-template-columns: 1fr; }
  .hub-grid { grid-template-columns: 1fr; }
  .metric { grid-template-columns: 1fr; gap: 6px; }
  th { width: 150px; }
}
"""


def check_current(output_dir: Path, terminal: Terminal) -> int:
    manifest_path = output_dir / MANIFEST_NAME
    if not manifest_path.exists():
        terminal.error_log(f"Missing {manifest_path}. Run scripts/generate-pkg-pages.py before deploy.")
        return 1
    try:
        manifest = read_json(manifest_path)
    except json.JSONDecodeError as err:
        terminal.error_log(f"Invalid {manifest_path}: {err}")
        return 1
    files = source_files()
    expected_hash, latest = source_digest(files)
    failures = validate_i18n_pkg_templates()
    if manifest.get("schema") != SCHEMA_VERSION:
        failures.append(f"schema is {manifest.get('schema')!r}, expected {SCHEMA_VERSION}")
    if manifest.get("source_hash") != expected_hash:
        failures.append("source hash does not match current data files")
    if manifest.get("latest_source_mtime_ns", 0) < latest:
        failures.append("generated package pages are older than a source file")
    page_count = int(manifest.get("page_count") or 0)
    actual_pages = sum(1 for path in output_dir.glob("*/*/index.html"))
    if actual_pages != page_count:
        failures.append(f"manifest page count is {page_count}, but found {actual_pages} pages")
    pages = sorted(package_pages_from_sources(load_sources()).values(), key=lambda page: (page.provider, page.slug, page.name))
    hubs = package_hub_pages(pages)
    indexable_pages = [page for page in pages if is_indexable_package_page(page)]
    noindex_pages = [page for page in pages if not is_indexable_package_page(page)]
    hub_count = int(manifest.get("hub_count") or 0)
    if hub_count != len(hubs):
        failures.append(f"manifest hub count is {hub_count}, but current data yields {len(hubs)} hubs")
    indexable_page_count = int(manifest.get("indexable_page_count") or 0)
    if indexable_page_count != len(indexable_pages):
        failures.append(f"manifest indexable page count is {indexable_page_count}, but current data yields {len(indexable_pages)}")
    isolated_pages = [page.key for page in indexable_pages if not has_internal_package_navigation(page)]
    if isolated_pages:
        failures.append(
            f"{len(isolated_pages):,} indexable package pages have no internal package graph links: {', '.join(isolated_pages[:12])}"
        )
    noindex_page_count = int(manifest.get("noindex_page_count") or 0)
    if noindex_page_count != len(noindex_pages):
        failures.append(f"manifest noindex page count is {noindex_page_count}, but current data yields {len(noindex_pages)}")
    markdown_page_count = int(manifest.get("markdown_page_count") or 0)
    if markdown_page_count != len(indexable_pages):
        failures.append(f"manifest markdown page count is {markdown_page_count}, but current data yields {len(indexable_pages)}")
    for page in indexable_pages:
        if not (output_dir / page.provider / page.slug / "index.md").exists():
            failures.append(f"missing package markdown alternate: {output_dir / page.provider / page.slug / 'index.md'}")
            break
    for page in noindex_pages:
        if (output_dir / page.provider / page.slug / "index.md").exists():
            failures.append(f"noindex package page has markdown alternate: {page.key}")
            break
    for locale in non_default_i18n_locales():
        localized_output_dir = output_dir.parent / locale_slug(locale) / "pkg"
        if not (localized_output_dir / "index.html").exists():
            failures.append(f"missing localized package index: {localized_output_dir / 'index.html'}")
            continue
        if not (localized_output_dir / "styles.css").exists():
            failures.append(f"missing localized package stylesheet: {localized_output_dir / 'styles.css'}")
        for page in indexable_pages[:12]:
            localized_page = localized_output_dir / page.provider / page.slug / "index.html"
            localized_markdown = localized_output_dir / page.provider / page.slug / "index.md"
            if not localized_page.exists():
                failures.append(f"missing localized package page: {localized_page}")
                break
            page_text = localized_page.read_text(encoding="utf-8")
            expected_canonical = locale_url(page.path, locale)
            if f'<html lang="{locale.get("htmlLang")}"' not in page_text:
                failures.append(f"localized page has wrong html lang: {localized_page}")
                break
            if f'<link rel="canonical" href="{expected_canonical}">' not in page_text:
                failures.append(f"localized page has wrong canonical: {localized_page}")
                break
            leaks = localized_package_ui_leaks(page_text)
            if leaks:
                failures.append(f"localized page has English package UI copy: {localized_page} ({', '.join(leaks[:4])})")
                break
            if not localized_markdown.exists():
                failures.append(f"missing localized package markdown alternate: {localized_markdown}")
                break
    sitemap_path = output_dir / "sitemap.xml"
    expected_sitemap_names = ["sitemap-hubs.xml"] + [
        f"sitemap-{provider}.xml"
        for provider in ("brew", "cask", "npm", "pip")
        if any(page.provider == provider for page in indexable_pages)
    ]
    if int(manifest.get("sitemap_count") or 0) != len(expected_sitemap_names):
        failures.append(f"manifest sitemap count is {manifest.get('sitemap_count')}, but current data yields {len(expected_sitemap_names)}")
    sitemap_page_counts = manifest.get("sitemap_page_counts") or {}
    if not isinstance(sitemap_page_counts, dict):
        failures.append("manifest sitemap page counts are missing or invalid")
        sitemap_page_counts = {}
    for provider in ("brew", "cask", "npm", "pip"):
        expected_provider_count = sum(1 for page in indexable_pages if page.provider == provider)
        if expected_provider_count and int(sitemap_page_counts.get(provider) or 0) != expected_provider_count:
            failures.append(
                f"manifest {provider} sitemap count is {sitemap_page_counts.get(provider)}, but current data yields {expected_provider_count}"
            )
    if sitemap_path.exists():
        sitemap = sitemap_path.read_text(encoding="utf-8")
        if "<sitemapindex" not in sitemap:
            failures.append(f"package sitemap root is not a sitemap index: {sitemap_path}")
        for name in expected_sitemap_names:
            provider_sitemap = output_dir / name
            if not provider_sitemap.exists():
                failures.append(f"missing package sitemap: {provider_sitemap}")
                continue
            if f"{SITE_ORIGIN}/pkg/{name}" not in sitemap:
                failures.append(f"package sitemap index does not reference {name}")
            provider_sitemap_text = provider_sitemap.read_text(encoding="utf-8")
            if 'xmlns:xhtml="http://www.w3.org/1999/xhtml"' not in provider_sitemap_text:
                failures.append(f"package sitemap lacks xhtml namespace: {provider_sitemap}")
            for locale in non_default_i18n_locales():
                sample = next((page for page in indexable_pages if page.provider in name), None)
                if sample and locale_url(sample.path, locale) not in provider_sitemap_text:
                    failures.append(f"package sitemap lacks {locale_code(locale)} alternate: {provider_sitemap}")
                    break
            for page in noindex_pages:
                if f"{SITE_ORIGIN}{page.path}" in provider_sitemap_text:
                    failures.append(f"noindex package page is present in sitemap: {page.key}")
                    break
    else:
        failures.append(f"missing package sitemap: {sitemap_path}")
    for hub, _hub_pages in hubs:
        if not (output_dir / hub.slug / "index.html").exists():
            failures.append(f"missing package hub page: {output_dir / hub.slug / 'index.html'}")
    if failures:
        terminal.error_log("Package SEO pages are stale.")
        for failure in failures:
            terminal.log(f"  - {failure}")
        terminal.log(f"{terminal.dim}Run scripts/generate-pkg-pages.py and retry deploy-www.{terminal.reset}")
        return 1
    terminal.ok_log(f"Package SEO pages are current ({fmt_int(page_count)} pages, {fmt_int(len(noindex_pages))} noindex)")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate crawlable pages for Nucleus packages.")
    parser.add_argument("--check", action="store_true", help="Validate that generated package pages match local data sources.")
    parser.add_argument("--output", default=str(OUTPUT_DIR), help=f"Output directory. Defaults to {OUTPUT_DIR}.")
    parser.add_argument("--json", action="store_true", help="Print machine-readable status and disable terminal styling.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    ensure_cwd()
    terminal = Terminal(json_mode=args.json)
    output_dir = Path(args.output)

    if args.check:
        return check_current(output_dir, terminal)

    terminal.header("Generating package SEO pages", "Local package metadata -> static HTML under www/pkg")
    terminal.step_log("Reading package sources")
    sources = load_sources()
    pages = package_pages_from_sources(sources)
    if not pages:
        terminal.error_log("No package metadata found in data/.")
        return 1
    files = source_files()
    manifest = build_manifest(len(pages), files)
    terminal.ok_log(f"Loaded {fmt_int(len(pages))} packages from {fmt_int(len(files))} source files")
    terminal.step_log("Rendering HTML, CSS, sitemap, and freshness manifest")
    render_all(pages, manifest, output_dir)
    terminal.ok_log(f"Wrote {fmt_int(len(pages))} package pages to {output_dir} ({fmt_int(manifest['noindex_page_count'])} noindex)")
    if args.json:
        print(json.dumps({"ok": True, "output": str(output_dir), "page_count": len(pages), "source_file_count": len(files)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
