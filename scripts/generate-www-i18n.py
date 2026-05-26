#!/usr/bin/env python3
import argparse
import copy
import html
import json
import re
import sys
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path
from typing import Any


SITE_ORIGIN = "https://www.automicvault.com"
LOCALES_PATH = Path("data/www-i18n/locales.json")
STATIC_PATH = Path("data/www-i18n/static/pages.json")
SITE_DIR = Path("www")
SITEMAP_PATH = SITE_DIR / "sitemap.xml"
I18N_SCRIPT = SITE_DIR / "i18n.js"


@dataclass(frozen=True)
class Locale:
    code: str
    slug: str
    html_lang: str
    hreflang: str
    display_name: str
    native_name: str
    browser_languages: tuple[str, ...]
    enabled: bool


TOPICS: dict[str, dict[str, dict[str, Any]]] = {
    "security": {
        "ja": {"title": "Automic Vault セキュリティ", "description": "Automic Vault のローカル実行境界、シークレット保存、承認ゲート、AI エージェント向け脅威モデル。", "h1": "AI エージェントのローカル権限を制限する", "sections": [["脅威モデル", "AI エージェントは端末、CLI、設定ファイルに触れるため、認証情報と高リスク操作を分離する必要があります。"], ["ローカル優先", "Automic Vault は macOS 上でシークレットを扱い、承認された実行だけに必要な値を渡します。"]]},
        "de": {"title": "Automic Vault Sicherheit", "description": "Lokale Laufzeitgrenzen, Secret-Speicherung, Approval Gates und Threat Model für AI-Agents in Automic Vault.", "h1": "Lokale Rechte von AI-Agents begrenzen", "sections": [["Threat Model", "AI-Agents können Terminals, CLIs und Konfigurationsdateien berühren; Credentials und riskante Aktionen brauchen Trennung."], ["Lokal zuerst", "Automic Vault verarbeitet Secrets auf macOS und gibt Werte nur an genehmigte Ausführungen weiter."]]},
        "fr": {"title": "Sécurité Automic Vault", "description": "Limites d'exécution locales, stockage des secrets, portes d'approbation et modèle de menace pour agents IA.", "h1": "Limiter l'autorité locale des agents IA", "sections": [["Modèle de menace", "Les agents IA peuvent toucher terminaux, CLI et fichiers de configuration; les identifiants et actions risquées doivent être séparés."], ["Local d'abord", "Automic Vault traite les secrets sur macOS et ne transmet les valeurs qu'aux exécutions approuvées."]]},
        "zh-Hans": {"title": "Automic Vault 安全", "description": "Automic Vault 的本地运行边界、密钥存储、审批门和面向 AI 代理的威胁模型。", "h1": "限制 AI 代理的本地权限", "sections": [["威胁模型", "AI 代理可能接触终端、CLI 和配置文件，因此凭据与高风险操作需要隔离。"], ["本地优先", "Automic Vault 在 macOS 上处理密钥，只把必要值传递给已批准的执行。"]]},
    },
    "privacy": {
        "ja": {"title": "Automic Vault プライバシー", "description": "Automic Vault のローカルデータ境界、ウェブサイト解析、プライバシー方針。", "h1": "ローカルシークレットはローカルに残る", "sections": [["製品データ", "Automic Vault は開発者マシン上のシークレットを保護するために作られており、ホスト型シークレットサービスではありません。"], ["サイトデータ", "ウェブサイトは基本的な解析と公開アセットを使い、製品シークレットを収集しません。"]]},
        "de": {"title": "Automic Vault Datenschutz", "description": "Lokale Datengrenzen, Website-Analytics und Datenschutznotizen für Automic Vault.", "h1": "Lokale Secrets bleiben lokal", "sections": [["Produktdaten", "Automic Vault schützt Secrets auf dem Entwicklergerät und ist kein gehosteter Secret-Dienst."], ["Websitedaten", "Die Website nutzt grundlegende Analytics und öffentliche Assets, sammelt aber keine Produkt-Secrets."]]},
        "fr": {"title": "Confidentialité Automic Vault", "description": "Limites de données locales, analytics du site et notes de confidentialité pour Automic Vault.", "h1": "Les secrets locaux restent locaux", "sections": [["Données produit", "Automic Vault protège les secrets sur la machine du développeur et n'est pas un service de secrets hébergé."], ["Données du site", "Le site utilise des analytics de base et des ressources publiques, sans collecter les secrets du produit."]]},
        "zh-Hans": {"title": "Automic Vault 隐私", "description": "Automic Vault 的本地数据边界、网站分析和隐私说明。", "h1": "本地密钥留在本地", "sections": [["产品数据", "Automic Vault 用于保护开发者机器上的密钥，并不是托管密钥服务。"], ["网站数据", "网站使用基础分析和公开资源，不收集产品中的密钥。"]]},
    },
    "terms": {
        "ja": {"title": "Automic Vault 利用規約", "description": "Automic Vault の利用条件、オープンソースライセンス、ウェブサイト利用メモ。", "h1": "オープンソースのローカルセキュリティツール", "sections": [["ライセンス", "Automic Vault は Apache License 2.0 の下で提供されます。"], ["利用", "このサイトは製品情報、ドキュメント、パッケージメタデータを提供します。"]]},
        "de": {"title": "Automic Vault Bedingungen", "description": "Nutzungsbedingungen, Open-Source-Lizenz und Website-Hinweise für Automic Vault.", "h1": "Open-Source-Werkzeug für lokale Sicherheit", "sections": [["Lizenz", "Automic Vault wird unter der Apache License 2.0 bereitgestellt."], ["Nutzung", "Diese Website stellt Produktinformationen, Dokumentation und Paketmetadaten bereit."]]},
        "fr": {"title": "Conditions Automic Vault", "description": "Conditions d'utilisation, licence open source et notes du site pour Automic Vault.", "h1": "Outil open source de sécurité locale", "sections": [["Licence", "Automic Vault est fourni sous licence Apache License 2.0."], ["Utilisation", "Ce site fournit des informations produit, de la documentation et des métadonnées de paquets."]]},
        "zh-Hans": {"title": "Automic Vault 条款", "description": "Automic Vault 的使用条款、开源许可证和网站说明。", "h1": "开源本地安全工具", "sections": [["许可证", "Automic Vault 以 Apache License 2.0 提供。"], ["使用", "本网站提供产品信息、文档和软件包元数据。"]]},
    },
}

ALIASED_TOPIC = {
    "pricing": {"ja": ("Automic Vault 価格", "Automic Vault は無料のオープンソースソフトウェアです。"), "de": ("Automic Vault Preise", "Automic Vault ist freie Open-Source-Software."), "fr": ("Tarifs Automic Vault", "Automic Vault est un logiciel open source gratuit."), "zh-Hans": ("Automic Vault 定价", "Automic Vault 是免费的开源软件。")},
    "download": {"ja": ("Automic Vault ダウンロード", "macOS 用 Automic Vault を入手し、ローカルの AI エージェント実行を保護します。"), "de": ("Automic Vault Download", "Lade Automic Vault für macOS herunter und schütze lokale AI-Agent-Läufe."), "fr": ("Télécharger Automic Vault", "Téléchargez Automic Vault pour macOS et protégez les exécutions locales d'agents IA."), "zh-Hans": ("下载 Automic Vault", "获取 macOS 版 Automic Vault，保护本地 AI 代理运行。")},
    "secretsManager": {"ja": ("AI エージェント向けシークレットマネージャー", "AI エージェントが平文ファイルを読まずに必要な認証情報を使えるようにします。"), "de": ("Secrets Manager für AI-Agents", "AI-Agents erhalten benötigte Credentials, ohne Klartextdateien lesen zu müssen."), "fr": ("Gestionnaire de secrets pour agents IA", "Les agents IA obtiennent les identifiants nécessaires sans lire les fichiers en clair."), "zh-Hans": ("面向 AI 代理的密钥管理器", "让 AI 代理无需读取明文文件也能使用必要凭据。")},
    "dotenv": {"ja": ("AI エージェントに .env を読ませない", ".env の常時露出を、承認されたツールへの制御された注入に置き換えます。"), "de": ("Verhindere, dass AI-Agents .env lesen", "Ersetze ständig sichtbare .env-Dateien durch kontrollierte Injektion in genehmigte Tools."), "fr": ("Empêcher les agents IA de lire .env", "Remplacez l'exposition permanente de .env par une injection contrôlée dans les outils approuvés."), "zh-Hans": ("阻止 AI 代理读取 .env", "用向已批准工具的受控注入替代持续暴露的 .env 文件。")},
    "apiKeys": {"ja": ("AI エージェント向け API キー管理", "CLI と SDK のトークンをモデルコンテキストや平文設定から遠ざけます。"), "de": ("API-Key-Management für AI-Agents", "Halte CLI- und SDK-Tokens aus Modellkontext und Klartextkonfiguration heraus."), "fr": ("Gestion des clés API pour agents IA", "Gardez les jetons CLI et SDK hors du contexte modèle et de la configuration en clair."), "zh-Hans": ("面向 AI 代理的 API 密钥管理", "让 CLI 与 SDK 令牌远离模型上下文和明文配置。")},
    "hashicorp": {"ja": ("AI エージェント向け HashiCorp Vault 補完", "Automic Vault は、エンタープライズ Vault の前にあるローカル実行レイヤーとして動作します。"), "de": ("HashiCorp Vault für AI-Agents ergänzen", "Automic Vault arbeitet als lokale Laufzeitschicht vor einem Enterprise Vault."), "fr": ("Compléter HashiCorp Vault pour agents IA", "Automic Vault agit comme couche d'exécution locale devant un coffre-fort d'entreprise."), "zh-Hans": ("补充面向 AI 代理的 HashiCorp Vault", "Automic Vault 作为企业 Vault 前面的本地运行层。")},
    "mcp": {"ja": ("MCP シークレット管理", "MCP ツールが必要な認証情報を、明示的な承認境界の中で受け取れるようにします。"), "de": ("MCP-Secret-Management", "MCP-Tools erhalten benötigte Credentials innerhalb klarer Freigabegrenzen."), "fr": ("Gestion des secrets MCP", "Les outils MCP reçoivent les identifiants nécessaires dans des limites d'approbation explicites."), "zh-Hans": ("MCP 密钥管理", "MCP 工具在明确审批边界内获取所需凭据。")},
    "pam": {"ja": ("AI エージェント向け特権アクセス管理", "ローカル開発ツールの危険な権限を、実行時の承認で制御します。"), "de": ("Privileged Access Management für AI-Agents", "Kontrolliere riskante Rechte lokaler Entwicklertools mit Laufzeitfreigaben."), "fr": ("Gestion des accès privilégiés pour agents IA", "Contrôlez les droits risqués des outils locaux avec des approbations à l'exécution."), "zh-Hans": ("面向 AI 代理的特权访问管理", "通过运行时审批控制本地开发工具的高风险权限。")},
    "approvalGates": {"ja": ("AI エージェント承認ゲート", "公開、削除、シークレット表示などの操作を実行前に確認します。"), "de": ("Approval Gates für AI-Agents", "Prüfe Veröffentlichung, Löschung und Secret-Ausgabe vor der Ausführung."), "fr": ("Portes d'approbation pour agents IA", "Vérifiez publication, suppression et affichage de secrets avant exécution."), "zh-Hans": ("AI 代理审批门", "在执行前确认发布、删除和密钥显示等操作。")},
    "awsCli": {"ja": ("AI エージェント向け AWS CLI 認証情報保護", "AWS 認証情報を平文設定から外し、承認された aws 実行だけに渡します。"), "de": ("AWS-CLI-Credentials für AI-Agents schützen", "Entferne AWS-Credentials aus Klartextkonfiguration und gib sie nur an genehmigte aws-Läufe weiter."), "fr": ("Sécuriser les identifiants AWS CLI pour agents IA", "Retirez les identifiants AWS de la configuration en clair et transmettez-les seulement aux exécutions aws approuvées."), "zh-Hans": ("保护 AI 代理的 AWS CLI 凭据", "将 AWS 凭据移出明文配置，只传递给已批准的 aws 执行。")},
    "githubCli": {"ja": ("AI エージェント向け GitHub CLI トークン保護", "ソース、リリース、パッケージ公開に使う gh トークンをエージェントから守ります。"), "de": ("GitHub-CLI-Token für AI-Agents schützen", "Schütze gh-Tokens für Source, Releases und Paketveröffentlichung vor Agents."), "fr": ("Sécuriser les jetons GitHub CLI pour agents IA", "Protégez les jetons gh utilisés pour source, releases et publication de paquets."), "zh-Hans": ("保护 AI 代理的 GitHub CLI 令牌", "保护用于源码、发布和软件包发布的 gh 令牌。")},
    "secretScanner": {"ja": ("AI エージェントシークレットスキャナー", "エージェント実行前にローカル環境の漏えいしやすい認証情報を見つけます。"), "de": ("Secret Scanner für AI-Agents", "Finde exponierte lokale Credentials, bevor ein Agent läuft."), "fr": ("Scanner de secrets pour agents IA", "Trouvez les identifiants locaux exposés avant l'exécution d'un agent."), "zh-Hans": ("AI 代理密钥扫描器", "在代理运行前发现本地环境中容易泄露的凭据。")},
    "avTrace": {"ja": ("シェルインストーラートレース", "curl | sh 形式のインストーラーを実行前に確認します。"), "de": ("Shell-Installer-Tracing", "Prüfe Installer im Stil curl | sh, bevor sie ausgeführt werden."), "fr": ("Traçage des installateurs shell", "Examinez les installateurs de type curl | sh avant leur exécution."), "zh-Hans": ("Shell 安装器追踪", "在执行前检查 curl | sh 形式的安装器。")},
    "scannerVsProtection": {"ja": ("シークレットスキャンとエージェント保護の違い", "検出だけではなく、実行時にシークレットへのアクセスを防ぐ理由を説明します。"), "de": ("Secret Scanning vs. Agent-Schutz", "Warum Laufzeitschutz den Secret-Zugriff verhindert, statt ihn nur zu erkennen."), "fr": ("Scan de secrets ou protection des agents", "Pourquoi la protection à l'exécution empêche l'accès aux secrets au lieu de seulement le détecter."), "zh-Hans": ("密钥扫描与代理保护", "说明为什么运行时保护不只是检测，而是阻止密钥访问。")},
}


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def enabled_locales() -> list[Locale]:
    data = load_json(LOCALES_PATH)
    locales = []
    for item in data["locales"]:
        if not item.get("enabled", False):
            continue
        locales.append(
            Locale(
                code=item["code"],
                slug=item["slug"],
                html_lang=item["htmlLang"],
                hreflang=item["hreflang"],
                display_name=item["displayName"],
                native_name=item["nativeName"],
                browser_languages=tuple(item.get("browserLanguages", [])),
                enabled=True,
            )
        )
    return locales


def non_default_locales() -> list[Locale]:
    return [locale for locale in enabled_locales() if locale.code != "en"]


def locale_path(path: str, locale: Locale | None) -> str:
    if locale is None or locale.code == "en":
        return path
    if path == "/":
        return f"/{locale.slug}/"
    return f"/{locale.slug}{path}"


def route_file(path: str, locale: Locale) -> Path:
    route = locale_path(path, locale).strip("/")
    if not route:
        return SITE_DIR / "index.html"
    return SITE_DIR / route / "index.html"


def rel_root(path: str, locale: Locale) -> str:
    depth = 1 if path == "/" else len(path.strip("/").split("/")) + 1
    return "../" * depth


def href(path: str, locale: Locale | None = None) -> str:
    return SITE_ORIGIN + locale_path(path, locale)


def alternate_link_block(path: str, locales: list[Locale], indent: str = "  ") -> str:
    links = [f'{indent}<link rel="alternate" hreflang="en" href="{href(path)}">']
    for locale in locales:
        if locale.code == "en":
            continue
        links.append(f'{indent}<link rel="alternate" hreflang="{locale.hreflang}" href="{href(path, locale)}">')
    links.append(f'{indent}<link rel="alternate" hreflang="x-default" href="{href(path)}">')
    return "\n".join(links)


def language_links(path: str, current: Locale, locales: list[Locale]) -> str:
    links = [f'<a href="{html.escape(locale_path(path, locale if locale.code != "en" else None))}" lang="{html.escape(locale.html_lang)}">{html.escape(locale.native_name)}</a>' for locale in locales]
    return f'<nav class="language-links" aria-label="Language versions">{" ".join(links)}</nav>'


def translated_page_records() -> list[dict[str, Any]]:
    data = load_json(STATIC_PATH)
    records = [copy.deepcopy(item) for item in data["pages"]]
    seed = records[0]
    aliases = data.get("aliases", {})
    for path, topic_key in aliases.items():
        if topic_key in TOPICS:
            translations = TOPICS[topic_key]
        else:
            translations = {}
            for locale_code, (title, lede) in ALIASED_TOPIC[topic_key].items():
                translations[locale_code] = {
                    "title": title,
                    "description": lede,
                    "kicker": title,
                    "h1": title,
                    "lede": lede,
                    "sections": [
                        [title, lede],
                        ["Automic Vault", generic_second_paragraph(locale_code)],
                    ],
                }
        records.append({
            "path": path,
            "source": path.strip("/") + "/index.html",
            "dateModified": seed.get("dateModified", "2026-05-24"),
            "translations": translations,
        })
    return records


def generic_second_paragraph(locale_code: str) -> str:
    return {
        "ja": "ローカルのシークレット、CLI、パッケージ実行を制御し、AI エージェントの権限を明確な境界の中に収めます。",
        "de": "Es kontrolliert lokale Secrets, CLIs und Paketausführung, damit AI-Agents innerhalb klarer Grenzen arbeiten.",
        "fr": "Il contrôle les secrets locaux, les CLI et l'exécution des paquets afin que les agents IA restent dans des limites claires.",
        "zh-Hans": "它控制本地密钥、CLI 和软件包执行，让 AI 代理在清晰边界内运行。",
    }[locale_code]


def render_page(record: dict[str, Any], locale: Locale, locales: list[Locale]) -> str:
    path = record["path"]
    t = record["translations"][locale.code]
    root = rel_root(path, locale)
    canonical = href(path, locale)
    sections = "\n".join(
        f"""      <section class="i18n-section">
        <h2>{html.escape(title)}</h2>
        <p>{html.escape(body)}</p>
      </section>"""
        for title, body in t.get("sections", [])
    )
    return f"""<!DOCTYPE html>
<html lang="{html.escape(locale.html_lang)}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{html.escape(t["title"])}</title>
  <meta name="description" content="{html.escape(t["description"], quote=True)}">
  <meta name="robots" content="index,follow">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="Automic Vault">
  <meta property="og:title" content="{html.escape(t["title"], quote=True)}">
  <meta property="og:description" content="{html.escape(t["description"], quote=True)}">
  <meta property="og:url" content="{html.escape(canonical, quote=True)}">
  <meta property="og:image" content="{SITE_ORIGIN}/preview.jpg">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="{html.escape(t["title"], quote=True)}">
  <meta name="twitter:description" content="{html.escape(t["description"], quote=True)}">
  <meta name="twitter:image" content="{SITE_ORIGIN}/preview.jpg">
  <link rel="canonical" href="{html.escape(canonical, quote=True)}">
{alternate_link_block(path, locales)}
  <link rel="icon" href="{root}favicon.ico" sizes="16x16 32x32 48x48">
  <link rel="stylesheet" href="{root}styles.css">
  <link rel="stylesheet" href="{root}seo.css">
  <script type="application/ld+json">
  {{
    "@context": "https://schema.org",
    "@type": "WebPage",
    "url": "{canonical}",
    "name": {json.dumps(t["title"], ensure_ascii=False)},
    "description": {json.dumps(t["description"], ensure_ascii=False)},
    "inLanguage": "{locale.html_lang}",
    "isPartOf": {{"@type": "WebSite", "name": "Automic Vault", "url": "{SITE_ORIGIN}/"}}
  }}
  </script>
</head>
<body>
  <div class="site-shell i18n-page">
    <header class="masthead">
      <a class="brand" href="{locale_path('/', locale)}" aria-label="Automic Vault home">
        <img class="brand-mark" src="/assets/icon@2x.webp" alt="" width="54" height="54">
        <span class="brand-type">Automic Vault</span>
      </a>
      <nav class="nav" aria-label="Main navigation">
        <a href="{locale_path('/docs/', locale)}">Docs</a>
        <a href="{locale_path('/security/', locale)}">Security</a>
        <a href="{locale_path('/pkg/', locale)}">Packages</a>
        <a href="https://github.com/automic-vault/">GitHub</a>
      </nav>
    </header>
    <main>
      <section class="hero i18n-hero">
        <p class="eyebrow">{html.escape(t.get("kicker", "Automic Vault"))}</p>
        <h1>{html.escape(t["h1"])}</h1>
        <p class="lede">{html.escape(t.get("lede", t["description"]))}</p>
        <div class="hero-actions">
          <a class="button primary" href="{locale_path('/download/', locale)}">Download</a>
          <a class="button secondary" href="{locale_path('/docs/', locale)}">Docs</a>
        </div>
      </section>
{sections}
      {language_links(path, locale, locales)}
    </main>
    <footer class="site-footer">
      <p>Automic Vault</p>
      <div class="footer-links">
        <a href="{locale_path('/privacy/', locale)}">Privacy</a>
        <a href="{locale_path('/terms/', locale)}">Terms</a>
        <a href="{locale_path('/llms.txt', locale)}">llms.txt</a>
      </div>
    </footer>
  </div>
</body>
</html>
"""


def render_llms(locale: Locale) -> str:
    lines = {
        "ja": ["# Automic Vault", "Automic Vault は macOS 上の AI エージェント向けローカルセキュリティレイヤーです。", "シークレットを平文ファイルから離し、承認されたツールだけに渡します。"],
        "de": ["# Automic Vault", "Automic Vault ist eine lokale Sicherheitsschicht für AI-Agents auf macOS.", "Secrets verlassen Klartextdateien und werden nur an genehmigte Tools weitergegeben."],
        "fr": ["# Automic Vault", "Automic Vault est une couche de sécurité locale pour agents IA sur macOS.", "Les secrets quittent les fichiers en clair et ne sont transmis qu'aux outils approuvés."],
        "zh-Hans": ["# Automic Vault", "Automic Vault 是 macOS 上面向 AI 代理的本地安全层。", "密钥不再保存在明文文件中，只会传递给已批准的工具。"],
    }[locale.code]
    return "\n\n".join(lines) + f"\n\n- Website: {href('/', locale)}\n- Packages: {href('/pkg/', locale)}\n"


def render_i18n_js(locales: list[Locale]) -> str:
    data = [
        {
            "code": locale.code,
            "slug": locale.slug,
            "nativeName": locale.native_name,
            "languages": list(locale.browser_languages),
        }
        for locale in locales
        if locale.code != "en"
    ]
    return f"""(() => {{
  const locales = {json.dumps(data, ensure_ascii=False)};
  const dismissedKey = "av-i18n-dismissed";
  if (localStorage.getItem(dismissedKey) === "1") return;
  const path = window.location.pathname;
  if (/^\\/(ja|de|fr|zh-hans)(\\/|$)/.test(path)) return;
  const languages = navigator.languages || [navigator.language || ""];
  const match = languages
    .map((item) => String(item).toLowerCase())
    .map((item) => locales.find((locale) => locale.languages.includes(item) || locale.languages.includes(item.split("-")[0])))
    .find(Boolean);
  if (!match) return;
  const localized = "/" + match.slug + (path === "/" ? "/" : path);
  fetch(localized, {{ method: "HEAD" }})
    .then((response) => {{
      if (!response.ok) return;
      const banner = document.createElement("aside");
      banner.className = "i18n-suggestion";
      banner.setAttribute("aria-label", "Language suggestion");
      banner.innerHTML = `<a href="${{localized}}">Read this page in ${{match.nativeName}}</a><button type="button" aria-label="Dismiss language suggestion">×</button>`;
      banner.querySelector("button").addEventListener("click", () => {{
        localStorage.setItem(dismissedKey, "1");
        banner.remove();
      }});
      document.body.appendChild(banner);
    }})
    .catch(() => {{}});
}})();
"""


def patch_english_page(path: str, locales: list[Locale], check: bool, failures: list[str]) -> None:
    file = route_file(path, Locale("en", "", "en", "en", "English", "English", ("en",), True))
    if not file.exists():
        failures.append(f"missing English source page: {file}")
        return
    text = file.read_text(encoding="utf-8")
    canonical_match = re.search(r'<link rel="canonical" href="https://www\.automicvault\.com([^"]*)">', text)
    if not canonical_match:
        failures.append(f"missing canonical in {file}")
        return
    route = canonical_match.group(1) or "/"
    block = alternate_link_block(route, locales)
    text = re.sub(r'\n  <link rel="alternate" hreflang="[^"]+" href="[^"]+">', "", text)
    if block not in text:
        text = text.replace(canonical_match.group(0), canonical_match.group(0) + "\n" + block)
    language_block = language_links(route, Locale("en", "", "en", "en", "English", "English", ("en",), True), locales)
    if "class=\"language-links\"" not in text:
        text = text.replace("</body>", f"  {language_block}\n  <script src=\"/i18n.js\" defer></script>\n</body>")
    if check:
        current = file.read_text(encoding="utf-8")
        if current != text:
            failures.append(f"stale i18n head/body metadata: {file}")
    else:
        file.write_text(text, encoding="utf-8")


def sitemap_entry(loc: str, lastmod: str, path: str | None, locales: list[Locale]) -> str:
    body = [f"  <url>", f"    <loc>{html.escape(loc)}</loc>", f"    <lastmod>{lastmod}</lastmod>"]
    if path:
        body.append(f'    <xhtml:link rel="alternate" hreflang="en" href="{href(path)}" />')
        for locale in locales:
            if locale.code == "en":
                continue
            body.append(f'    <xhtml:link rel="alternate" hreflang="{locale.hreflang}" href="{href(path, locale)}" />')
        body.append(f'    <xhtml:link rel="alternate" hreflang="x-default" href="{href(path)}" />')
    body.append("  </url>")
    return "\n".join(body)


def render_sitemap(records: list[dict[str, Any]], locales: list[Locale]) -> str:
    entries: list[str] = []
    for record in records:
        path = record["path"]
        lastmod = record.get("dateModified", "2026-05-24")
        entries.append(sitemap_entry(href(path), lastmod, path, locales))
        for locale in locales:
            if locale.code == "en":
                continue
            entries.append(sitemap_entry(href(path, locale), lastmod, path, locales))
    preserved = [
        ("https://www.automicvault.com/llms.txt", "2026-05-24"),
        ("https://www.automicvault.com/llms-full.txt", "2026-05-24"),
        ("https://www.automicvault.com/pricing.md", "2026-05-24"),
        ("https://www.automicvault.com/.well-known/security.txt", "2026-05-24"),
    ]
    entries.extend(sitemap_entry(loc, lastmod, None, locales) for loc, lastmod in preserved)
    return '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n' + "\n".join(entries) + "\n</urlset>\n"


def generate(check: bool = False) -> int:
    locales = enabled_locales()
    locale_codes = {locale.code for locale in locales}
    records = translated_page_records()
    failures: list[str] = []
    for record in records:
        missing = locale_codes - {"en"} - set(record.get("translations", {}).keys())
        if missing:
            failures.append(f"{record['path']} missing translations: {', '.join(sorted(missing))}")
            continue
        patch_english_page(record["path"], locales, check, failures)
        for locale in non_default_locales():
            output = route_file(record["path"], locale)
            expected = render_page(record, locale, locales)
            if check:
                if not output.exists() or output.read_text(encoding="utf-8") != expected:
                    failures.append(f"stale localized page: {output}")
            else:
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_text(expected, encoding="utf-8")
    for locale in non_default_locales():
        output = SITE_DIR / locale.slug / "llms.txt"
        expected = render_llms(locale)
        if check:
            if not output.exists() or output.read_text(encoding="utf-8") != expected:
                failures.append(f"stale localized llms.txt: {output}")
        else:
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_text(expected, encoding="utf-8")
    expected_js = render_i18n_js(locales)
    if check:
        if not I18N_SCRIPT.exists() or I18N_SCRIPT.read_text(encoding="utf-8") != expected_js:
            failures.append(f"stale i18n browser helper: {I18N_SCRIPT}")
    else:
        I18N_SCRIPT.write_text(expected_js, encoding="utf-8")
    expected_sitemap = render_sitemap(records, locales)
    if check:
        if not SITEMAP_PATH.exists() or SITEMAP_PATH.read_text(encoding="utf-8") != expected_sitemap:
            failures.append(f"stale localized sitemap: {SITEMAP_PATH}")
    else:
        SITEMAP_PATH.write_text(expected_sitemap, encoding="utf-8")
    if failures:
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate localized static website pages.")
    parser.add_argument("--check", action="store_true", help="Validate generated localized static pages.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parents[1]
    import os
    os.chdir(root)
    return generate(check=args.check)


if __name__ == "__main__":
    raise SystemExit(main())
