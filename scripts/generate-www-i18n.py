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

HOME_DETAIL: dict[str, dict[str, Any]] = {
    "ja": {
        "meta": ["macOS", "ローカル優先", "エージェント実行時セキュリティ", "2026年5月24日更新"],
        "brief": [
            "シークレットは、承認されたツールが必要とするまで Keychain-backed storage に残ります。",
            "危険なツール操作は、実行時に人間の承認を要求できます。",
            "リリース版のインストールは /opt に入り、/usr/local/bin のスタブから起動します。",
        ],
        "nav": ["境界", "シークレット", "承認", "Nucleus", "パッケージ", "ドキュメント", "ダウンロード"],
        "actions": [".dmg をダウンロード", "ドキュメントを読む", "スキャナーを実行"],
        "highlights": [
            ["01 / secrets", "エージェントが読み取れる平文の認証情報ファイルをなくします。"],
            ["02 / approval", "機密性の高いツール操作が実行される場所に承認を置きます。"],
            ["03 / packages", "エージェントのツールチェーンに強化された root と依存スタックを与えます。"],
            ["04 / trace", "curl-pipe-shell インストーラーがファイルを書き込む前に調べます。"],
        ],
        "storiesTitle": "主要な境界",
        "storiesLede": "エージェントが Mac 上でツールを実行できるときに変わること。",
        "stories": [
            ["Keychain ベースのシークレット", "ツールはシークレットを受け取る。エージェントは受け取らない。", "Automic Vault は重要なツールに境界を追加し、認証情報を平文ファイルからローカルの保護ストレージへ移します。ツールは動き続け、エージェントは簡単な読み取り経路を失います。"],
            ["人間による承認ゲート", "承認はエージェントの内側ではなく下に置く。", "モデル内の制御も役立ちますが、侵害されたエージェントは自分のポリシー面を操作できます。Automic Vault はトークン出力、パッケージ公開、その他の機密操作が実行されるローカルツール層にゲートを置きます。"],
            ["Nucleus パッケージマネージャー", "エージェントのツールを、書き換えられない root にインストール。", "Nucleus は Homebrew、npm、PyPI パッケージを強化された root にインストールします。エージェントは承認済みツールを実行できますが、開発環境を自由に書き換えられる状態にはしません。"],
            ["平文露出スキャン", "実行前にエージェントから見えるものを探す。", "av secret-scanner は、ローカルファイルにすでに露出している認証情報を検索します。自律実行に広いファイルアクセスを渡す前の高速な事前確認に使えます。"],
            ["Automic Vault.app", "パッケージ制御のためのネイティブ Mac 画面。", "パッケージ検索、メタデータ確認、Touch ID での承認、アップデート確認を行い、端末が適した場面では av CLI を使えます。"],
        ],
        "fitTitle": "位置づけ",
        "fitKicker": "単なるラッパーではありません",
        "fit": [
            ["Homebrew", "パッケージマネージャー", "Automic Vault は馴染みのあるパッケージをインストールし、その下をエージェントが書き換えられる範囲を制限します。"],
            ["1Password", "シークレットマネージャー", "中央の vault はシークレットを管理します。Automic Vault は、ローカルツールがそのシークレットを受け取れるかを制御します。"],
            ["エージェント制御", "実行ポリシー", "エージェント側の制御は有用です。ツール層の制御は、モデルとプロンプトの下で残ります。"],
        ],
        "guidesTitle": "ガイド",
        "guidesKicker": "詳しい読み物",
        "radarTitle": "既知の Homebrew シークレット逃げ道を閉じる、または見える化する。",
        "radarText": "17,450 件の formula と tap 候補を確認済み。残る既知リスクは GUI の hazard として表示されます。",
        "final": "次の自律実行の前に、ツール層を保護する。",
    },
    "de": {
        "meta": ["macOS", "lokal zuerst", "Agent-Laufzeitsicherheit", "aktualisiert am 24. Mai 2026"],
        "brief": [
            "Secrets bleiben im Keychain-gestützten Speicher, bis ein genehmigtes Tool sie benötigt.",
            "Riskante Tool-Aktionen können zur Laufzeit menschliche Freigabe verlangen.",
            "Release-Installationen liegen unter /opt, mit stabilen Stubs in /usr/local/bin.",
        ],
        "nav": ["Grenzen", "Secrets", "Freigabe", "Nucleus", "Pakete", "Dokumentation", "Herunterladen"],
        "actions": [".dmg herunterladen", "Dokumentation lesen", "Scanner starten"],
        "highlights": [
            ["01 / secrets", "Keine Klartext-Credential-Datei, die Agents auslesen können."],
            ["02 / approval", "Freigaben sitzen dort, wo sensitive Tool-Aktionen ausgeführt werden."],
            ["03 / packages", "Agent-Toolchains erhalten gehärtete Roots und transitive Stacks."],
            ["04 / trace", "Prüfe curl-pipe-shell-Installer, bevor sie Dateien schreiben."],
        ],
        "storiesTitle": "Wichtige Grenzen",
        "storiesLede": "Was sich ändert, wenn ein Agent Tools auf deinem Mac ausführen kann.",
        "stories": [
            ["Keychain-gestützte Secrets", "Tools bekommen Secrets. Agents nicht.", "Automic Vault ergänzt kritische Tools, damit Credentials aus Klartextdateien in lokalen geschützten Speicher wandern. Das Tool funktioniert weiter; der Agent verliert den einfachen Lesepfad."],
            ["Menschliche Approval Gates", "Freigabe gehört unter den Agent, nicht in ihn hinein.", "Agent-interne Kontrollen helfen, aber ein kompromittierter Agent kontrolliert seine eigene Policy-Fläche. Automic Vault setzt Gates an die lokale Tool-Schicht, wo Token-Export, Paketveröffentlichung und andere sensitive Aktionen laufen."],
            ["Nucleus-Paketmanager", "Installiere Agent-Tools in eine Root, die er nicht umschreiben kann.", "Nucleus installiert Homebrew-, npm- und PyPI-Pakete in gehärtete Roots. Agents können genehmigte Tools ausführen, ohne die Entwicklerumgebung in beschreibbaren Umgebungszustand zu verwandeln."],
            ["Klartext-Exposure-Scan", "Finde, was ein Agent sehen kann, bevor du den Lauf startest.", "av secret-scanner sucht Credentials, die bereits in lokalen Dateien liegen. Nutze ihn als schnellen Preflight, bevor ein autonomer Lauf breiten Dateizugriff bekommt."],
            ["Automic Vault.app", "Eine native Mac-Oberfläche für Paketkontrolle.", "Suche Pakete, prüfe Metadaten, genehmige Installationen mit Touch ID, verfolge Updates und nutze die av CLI, wenn das Terminal die richtige Oberfläche ist."],
        ],
        "fitTitle": "Einordnung",
        "fitKicker": "nicht noch ein Wrapper",
        "fit": [
            ["Homebrew", "Paketmanager", "Automic Vault installiert bekannte Pakete und begrenzt danach, was Agents darunter umschreiben können."],
            ["1Password", "Secrets Manager", "Zentrale Vaults verwalten Secrets. Automic Vault kontrolliert, ob ein lokales Tool eines erhalten darf."],
            ["Agent-Kontrollen", "Ausführungsrichtlinie", "Agent-Kontrollen sind nützlich. Tool-Layer-Kontrollen bleiben unter Modell und Prompt bestehen."],
        ],
        "guidesTitle": "Guides",
        "guidesKicker": "Vertiefung",
        "radarTitle": "Bekannte Homebrew-Secret-Auswege, geschlossen oder sichtbar gemacht.",
        "radarText": "17.450 Formula- und Tap-Kandidaten geprüft; verbleibende bekannte Risiken erscheinen als GUI-Hazards.",
        "final": "Sichere die Tool-Schicht vor dem nächsten autonomen Lauf.",
    },
    "fr": {
        "meta": ["macOS", "local d'abord", "sécurité d'exécution des agents", "mis à jour le 24 mai 2026"],
        "brief": [
            "Les secrets restent dans un stockage adossé au trousseau jusqu'à ce que l'outil approuvé en ait besoin.",
            "Les actions dangereuses des outils peuvent exiger une approbation humaine au moment de l'exécution.",
            "Les installations de release vivent sous /opt, avec des stubs stables dans /usr/local/bin.",
        ],
        "nav": ["Limites", "Secrets", "Approbation", "Nucleus", "Paquets", "Documentation", "Télécharger"],
        "actions": ["Télécharger le .dmg", "Lire la doc", "Lancer le scanner"],
        "highlights": [
            ["01 / secrets", "Plus de fichier d'identifiants en clair que les agents peuvent aspirer."],
            ["02 / approval", "Les validations vivent là où les actions sensibles s'exécutent."],
            ["03 / packages", "Les toolchains d'agents obtiennent des racines durcies et des piles transitives."],
            ["04 / trace", "Inspectez les installateurs curl-pipe-shell avant qu'ils écrivent des fichiers."],
        ],
        "storiesTitle": "Limites principales",
        "storiesLede": "Ce qui change quand un agent peut exécuter des outils sur votre Mac.",
        "stories": [
            ["Secrets adossés au trousseau", "Les outils reçoivent les secrets. Les agents, non.", "Automic Vault ajoute une frontière aux outils critiques pour déplacer les identifiants hors des fichiers en clair vers un stockage local protégé. L'outil continue de fonctionner; l'agent perd le chemin de lecture facile."],
            ["Portes d'approbation humaines", "L'approbation doit vivre sous l'agent, pas en lui.", "Les contrôles intégrés aux agents aident, mais un agent compromis contrôle sa propre surface de politique. Automic Vault place les portes dans la couche locale des outils, là où s'exécutent l'export de jetons, la publication de paquets et les autres actions sensibles."],
            ["Gestionnaire de paquets Nucleus", "Installez les outils de l'agent dans une racine qu'il ne peut pas réécrire.", "Nucleus installe les paquets Homebrew, npm et PyPI dans des racines durcies. Les agents peuvent lancer les outils approuvés sans transformer l'environnement développeur en état ambiant modifiable."],
            ["Scan d'exposition en clair", "Trouvez ce qu'un agent peut voir avant de lancer l'exécution.", "av secret-scanner recherche les identifiants déjà exposés dans les fichiers locaux. Utilisez-le comme préflight rapide avant de donner un large accès au système de fichiers à une exécution autonome."],
            ["Automic Vault.app", "Une surface Mac native pour contrôler les paquets.", "Recherchez des paquets, inspectez les métadonnées, approuvez les installations avec Touch ID, suivez les mises à jour et utilisez la CLI av quand le terminal est la bonne interface."],
        ],
        "fitTitle": "Positionnement",
        "fitKicker": "pas un wrapper de plus",
        "fit": [
            ["Homebrew", "Gestionnaire de paquets", "Automic Vault installe des paquets familiers, puis limite ce que les agents peuvent réécrire sous eux."],
            ["1Password", "Gestionnaire de secrets", "Les coffres centraux gèrent les secrets. Automic Vault contrôle si un outil local peut en recevoir un."],
            ["Contrôles d'agent", "Politique d'exécution", "Les contrôles au niveau de l'agent sont utiles. Les contrôles au niveau outil survivent sous le modèle et son prompt."],
        ],
        "guidesTitle": "Guides",
        "guidesKicker": "lectures approfondies",
        "radarTitle": "Échappements de secrets Homebrew connus, fermés ou rendus visibles.",
        "radarText": "17 450 formules et taps candidats examinés; les risques connus restants apparaissent comme dangers dans l'interface.",
        "final": "Sécurisez la couche outil avant la prochaine exécution autonome.",
    },
    "zh-Hans": {
        "meta": ["macOS", "本地优先", "代理运行时安全", "2026 年 5 月 24 日更新"],
        "brief": [
            "密钥会留在 Keychain 支持的存储中，直到已批准的工具需要它们。",
            "危险工具操作可以在执行时要求人工审批。",
            "发布版安装位于 /opt，并通过 /usr/local/bin 中的稳定 stub 入口运行。",
        ],
        "nav": ["边界", "密钥", "审批", "Nucleus", "软件包", "文档", "下载"],
        "actions": ["下载 .dmg", "阅读文档", "运行扫描器"],
        "highlights": [
            ["01 / secrets", "不再有可被代理抓取的明文凭据文件。"],
            ["02 / approval", "审批位于敏感工具操作实际执行的位置。"],
            ["03 / packages", "代理工具链获得加固的 root 和传递依赖栈。"],
            ["04 / trace", "在 curl-pipe-shell 安装器写入文件前进行检查。"],
        ],
        "storiesTitle": "核心边界",
        "storiesLede": "当代理可以在你的 Mac 上运行工具时，真正改变的部分。",
        "stories": [
            ["Keychain 支持的密钥", "工具获得密钥。代理不会。", "Automic Vault 为关键工具加入边界，让凭据离开明文文件并进入本地受保护存储。工具继续工作，代理失去简单读取路径。"],
            ["人工审批门", "审批应位于代理之下，而不是代理内部。", "代理内置控制有帮助，但被攻破的代理会控制自己的策略面。Automic Vault 将门放在本地工具层，也就是令牌导出、软件包发布和其他敏感操作实际运行的位置。"],
            ["Nucleus 软件包管理器", "把代理工具安装到它无法重写的 root 中。", "Nucleus 将 Homebrew、npm 和 PyPI 软件包安装到加固 root。代理可以运行已批准工具，但不会把开发环境变成可随意写入的环境状态。"],
            ["明文暴露扫描", "运行前找出代理能看到什么。", "av secret-scanner 会搜索已经暴露在本地文件中的凭据。在给自主运行授予广泛文件访问前，可用它做快速预检。"],
            ["Automic Vault.app", "用于软件包控制的原生 Mac 界面。", "搜索软件包、检查元数据、用 Touch ID 批准安装、跟踪更新；当终端更合适时使用 av CLI。"],
        ],
        "fitTitle": "定位",
        "fitKicker": "不是又一个包装器",
        "fit": [
            ["Homebrew", "软件包管理器", "Automic Vault 安装熟悉的软件包，然后限制代理能在其下方重写什么。"],
            ["1Password", "密钥管理器", "中心化 vault 管理密钥。Automic Vault 控制本地工具是否能接收某个密钥。"],
            ["代理控制", "执行策略", "代理层控制很有用。工具层控制位于模型和提示词下方，仍然存在。"],
        ],
        "guidesTitle": "指南",
        "guidesKicker": "深入阅读",
        "radarTitle": "已知 Homebrew 密钥逃逸路径，已关闭或已显现。",
        "radarText": "已审查 17,450 个 formula 和 tap 候选；剩余已知风险会作为 GUI hazard 显示。",
        "final": "在下一次自主运行前保护工具层。",
    },
}

UI_COPY: dict[str, dict[str, str]] = {
    "en": {
        "about": "About",
        "approvalPrompt": "Agent wants to run",
        "approvalQuestion": "Approve?",
        "approvalRequestAria": "Example approval request",
        "approve": "Approve",
        "brandHomeAria": "Automic Vault home",
        "caseApproval": "AI agent approval gates",
        "caseAws": "Secure AWS CLI credentials",
        "caseFiles": "Case Files",
        "caseGithub": "GitHub CLI token security",
        "currentSecurityPostureAria": "Current security posture",
        "deny": "Deny",
        "dismissLanguageSuggestion": "Dismiss language suggestion",
        "docs": "Docs",
        "download": "Download",
        "finalKicker": "Free and open source",
        "highlights": "Highlights",
        "home": "Home",
        "languageSuggestionAria": "Language suggestion",
        "languageSuggestionText": "Read this page in English",
        "languageVersionsAria": "Language versions",
        "mainNavigationAria": "Main navigation",
        "operationalNotesAria": "Operational notes",
        "packageSourcesAria": "Package sources",
        "packages": "Packages",
        "privacy": "Privacy",
        "rankedFeaturesAria": "Automic Vault ranked features",
        "releaseLabel": "release",
        "releaseNote": "Root-owned package installs.",
        "runtime": "Runtime",
        "security": "Security",
        "securityMap": "security map",
        "secretBoundaryDetailsAria": "Secret boundary details",
        "screenshotAlt": "Automic Vault app showing package search and package details",
        "stableEntrypoints": "Stable command entrypoints.",
        "stubsLabel": "stubs",
        "terms": "Terms",
        "toggleNavigationAria": "Toggle navigation",
        "v0Surface": "v0 surface",
        "viewSource": "View source",
        "website": "Website",
    },
    "ja": {
        "about": "概要",
        "approvalPrompt": "エージェントが実行を要求しています:",
        "approvalQuestion": "承認しますか？",
        "approvalRequestAria": "承認リクエスト例",
        "approve": "承認",
        "brandHomeAria": "Automic Vault ホーム",
        "caseApproval": "AI エージェント承認ゲート",
        "caseAws": "AWS CLI 認証情報の保護",
        "caseFiles": "ケースファイル",
        "caseGithub": "GitHub CLI トークン保護",
        "currentSecurityPostureAria": "現在のセキュリティ状態",
        "deny": "拒否",
        "dismissLanguageSuggestion": "言語提案を閉じる",
        "docs": "ドキュメント",
        "download": "ダウンロード",
        "finalKicker": "無料のオープンソース",
        "highlights": "ハイライト",
        "home": "ホーム",
        "languageSuggestionAria": "言語の提案",
        "languageSuggestionText": "このページを日本語で読む",
        "languageVersionsAria": "言語版",
        "mainNavigationAria": "メインナビゲーション",
        "operationalNotesAria": "運用メモ",
        "packageSourcesAria": "パッケージソース",
        "packages": "パッケージ",
        "privacy": "プライバシー",
        "rankedFeaturesAria": "Automic Vault の主要機能",
        "releaseLabel": "リリース",
        "releaseNote": "root 所有のパッケージインストール。",
        "runtime": "実行環境",
        "security": "セキュリティ",
        "securityMap": "セキュリティマップ",
        "secretBoundaryDetailsAria": "シークレット境界の詳細",
        "screenshotAlt": "パッケージ検索と詳細を表示する Automic Vault アプリ",
        "stableEntrypoints": "安定したコマンド入口。",
        "stubsLabel": "スタブ",
        "terms": "利用規約",
        "toggleNavigationAria": "ナビゲーションを開閉",
        "v0Surface": "v0 対象範囲",
        "viewSource": "ソースを見る",
        "website": "ウェブサイト",
    },
    "de": {
        "about": "Über uns",
        "approvalPrompt": "Agent möchte ausführen:",
        "approvalQuestion": "Freigeben?",
        "approvalRequestAria": "Beispiel für Freigabeanfrage",
        "approve": "Freigeben",
        "brandHomeAria": "Automic Vault Startseite",
        "caseApproval": "Approval Gates für AI-Agents",
        "caseAws": "AWS-CLI-Credentials schützen",
        "caseFiles": "Fallbeispiele",
        "caseGithub": "GitHub-CLI-Token schützen",
        "currentSecurityPostureAria": "Aktueller Sicherheitsstatus",
        "deny": "Ablehnen",
        "dismissLanguageSuggestion": "Sprachvorschlag schließen",
        "docs": "Dokumentation",
        "download": "Herunterladen",
        "finalKicker": "Kostenlos und Open Source",
        "highlights": "Kernpunkte",
        "home": "Startseite",
        "languageSuggestionAria": "Sprachvorschlag",
        "languageSuggestionText": "Diese Seite auf Deutsch lesen",
        "languageVersionsAria": "Sprachversionen",
        "mainNavigationAria": "Hauptnavigation",
        "operationalNotesAria": "Betriebsnotizen",
        "packageSourcesAria": "Paketquellen",
        "packages": "Pakete",
        "privacy": "Datenschutz",
        "rankedFeaturesAria": "Automic Vault Hauptfunktionen",
        "releaseLabel": "Release",
        "releaseNote": "Paketinstallationen mit root-Besitz.",
        "runtime": "Laufzeit",
        "security": "Sicherheit",
        "securityMap": "Sicherheitskarte",
        "secretBoundaryDetailsAria": "Details zur Secret-Grenze",
        "screenshotAlt": "Automic Vault App mit Paketsuche und Paketdetails",
        "stableEntrypoints": "Stabile Befehlseinstiege.",
        "stubsLabel": "Stubs",
        "terms": "Bedingungen",
        "toggleNavigationAria": "Navigation umschalten",
        "v0Surface": "v0-Oberfläche",
        "viewSource": "Quellcode ansehen",
        "website": "Website",
    },
    "fr": {
        "about": "À propos",
        "approvalPrompt": "L'agent veut exécuter :",
        "approvalQuestion": "Approuver ?",
        "approvalRequestAria": "Exemple de demande d'approbation",
        "approve": "Approuver",
        "brandHomeAria": "Accueil Automic Vault",
        "caseApproval": "Portes d'approbation pour agents IA",
        "caseAws": "Identifiants AWS CLI sécurisés",
        "caseFiles": "Cas pratiques",
        "caseGithub": "Sécurité des jetons GitHub CLI",
        "currentSecurityPostureAria": "État de sécurité actuel",
        "deny": "Refuser",
        "dismissLanguageSuggestion": "Fermer la suggestion de langue",
        "docs": "Documentation",
        "download": "Télécharger",
        "finalKicker": "Gratuit et open source",
        "highlights": "Points forts",
        "home": "Accueil",
        "languageSuggestionAria": "Suggestion de langue",
        "languageSuggestionText": "Lire cette page en français",
        "languageVersionsAria": "Versions linguistiques",
        "mainNavigationAria": "Navigation principale",
        "operationalNotesAria": "Notes d'exploitation",
        "packageSourcesAria": "Sources des paquets",
        "packages": "Paquets",
        "privacy": "Confidentialité",
        "rankedFeaturesAria": "Fonctionnalités principales d'Automic Vault",
        "releaseLabel": "release",
        "releaseNote": "Installations de paquets détenues par root.",
        "runtime": "Exécution",
        "security": "Sécurité",
        "securityMap": "carte de sécurité",
        "secretBoundaryDetailsAria": "Détails de la limite des secrets",
        "screenshotAlt": "Application Automic Vault affichant la recherche et les détails de paquets",
        "stableEntrypoints": "Points d'entrée de commande stables.",
        "stubsLabel": "stubs",
        "terms": "Conditions",
        "toggleNavigationAria": "Afficher ou masquer la navigation",
        "v0Surface": "surface v0",
        "viewSource": "Voir le code source",
        "website": "Site web",
    },
    "zh-Hans": {
        "about": "关于",
        "approvalPrompt": "代理想要运行：",
        "approvalQuestion": "批准吗？",
        "approvalRequestAria": "审批请求示例",
        "approve": "批准",
        "brandHomeAria": "Automic Vault 首页",
        "caseApproval": "AI 代理审批门",
        "caseAws": "保护 AWS CLI 凭据",
        "caseFiles": "案例",
        "caseGithub": "GitHub CLI 令牌安全",
        "currentSecurityPostureAria": "当前安全状态",
        "deny": "拒绝",
        "dismissLanguageSuggestion": "关闭语言建议",
        "docs": "文档",
        "download": "下载",
        "finalKicker": "免费开源",
        "highlights": "亮点",
        "home": "首页",
        "languageSuggestionAria": "语言建议",
        "languageSuggestionText": "用简体中文阅读本页",
        "languageVersionsAria": "语言版本",
        "mainNavigationAria": "主导航",
        "operationalNotesAria": "运维备注",
        "packageSourcesAria": "软件包来源",
        "packages": "软件包",
        "privacy": "隐私",
        "rankedFeaturesAria": "Automic Vault 主要功能",
        "releaseLabel": "发布版",
        "releaseNote": "root 拥有的软件包安装。",
        "runtime": "运行时",
        "security": "安全",
        "securityMap": "安全地图",
        "secretBoundaryDetailsAria": "密钥边界详情",
        "screenshotAlt": "显示软件包搜索和详情的 Automic Vault 应用",
        "stableEntrypoints": "稳定的命令入口。",
        "stubsLabel": "stub",
        "terms": "条款",
        "toggleNavigationAria": "切换导航",
        "v0Surface": "v0 范围",
        "viewSource": "查看源码",
        "website": "网站",
    },
}

ALIASED_TOPIC = {
    "pricing": {"ja": ("Automic Vault 価格", "Automic Vault は無料のオープンソースソフトウェアです。"), "de": ("Automic Vault Preise", "Automic Vault ist freie Open-Source-Software."), "fr": ("Tarifs Automic Vault", "Automic Vault est un logiciel open source gratuit."), "zh-Hans": ("Automic Vault 定价", "Automic Vault 是免费的开源软件。")},
    "download": {"ja": ("Automic Vault ダウンロード", "macOS 用 Automic Vault を入手し、ローカルの AI エージェント実行を保護します。"), "de": ("Automic Vault herunterladen", "Lade Automic Vault für macOS herunter und schütze lokale AI-Agent-Läufe."), "fr": ("Télécharger Automic Vault", "Téléchargez Automic Vault pour macOS et protégez les exécutions locales d'agents IA."), "zh-Hans": ("下载 Automic Vault", "获取 macOS 版 Automic Vault，保护本地 AI 代理运行。")},
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


def ui_copy(locale_code: str) -> dict[str, str]:
    return UI_COPY.get(locale_code, UI_COPY["en"])


def alternate_link_block(path: str, locales: list[Locale], indent: str = "  ") -> str:
    links = [f'{indent}<link rel="alternate" hreflang="en" href="{href(path)}">']
    for locale in locales:
        if locale.code == "en":
            continue
        links.append(f'{indent}<link rel="alternate" hreflang="{locale.hreflang}" href="{href(path, locale)}">')
    links.append(f'{indent}<link rel="alternate" hreflang="x-default" href="{href(path)}">')
    return "\n".join(links)


def language_links(path: str, current: Locale, locales: list[Locale]) -> str:
    ui = ui_copy(current.code)
    links = [f'<a href="{html.escape(locale_path(path, locale if locale.code != "en" else None))}" lang="{html.escape(locale.html_lang)}">{html.escape(locale.native_name)}</a>' for locale in locales]
    return f'<nav class="language-links" aria-label="{html.escape(ui["languageVersionsAria"], quote=True)}">{" ".join(links)}</nav>'


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
    ui = ui_copy(locale.code)
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
      <a class="brand" href="{locale_path('/', locale)}" aria-label="{html.escape(ui["brandHomeAria"], quote=True)}">
        <img class="brand-mark" src="/assets/icon@2x.webp" alt="" width="54" height="54">
        <span class="brand-type">Automic Vault</span>
      </a>
      <nav class="nav" aria-label="{html.escape(ui["mainNavigationAria"], quote=True)}">
        <a href="{locale_path('/docs/', locale)}">{html.escape(ui["docs"])}</a>
        <a href="{locale_path('/security/', locale)}">{html.escape(ui["security"])}</a>
        <a href="{locale_path('/pkg/', locale)}">{html.escape(ui["packages"])}</a>
        <a href="https://github.com/automic-vault/">GitHub</a>
      </nav>
    </header>
    <main>
      <section class="hero i18n-hero">
        <p class="eyebrow">{html.escape(t.get("kicker", "Automic Vault"))}</p>
        <h1>{html.escape(t["h1"])}</h1>
        <p class="lede">{html.escape(t.get("lede", t["description"]))}</p>
        <div class="hero-actions">
          <a class="button primary" href="{locale_path('/download/', locale)}">{html.escape(ui["download"])}</a>
          <a class="button secondary" href="{locale_path('/docs/', locale)}">{html.escape(ui["docs"])}</a>
        </div>
      </section>
{sections}
      {language_links(path, locale, locales)}
    </main>
    <footer class="site-footer">
      <p>Automic Vault</p>
      <div class="footer-links">
        <a href="{locale_path('/privacy/', locale)}">{html.escape(ui["privacy"])}</a>
        <a href="{locale_path('/terms/', locale)}">{html.escape(ui["terms"])}</a>
        <a href="{locale_path('/llms.txt', locale)}">llms.txt</a>
      </div>
    </footer>
  </div>
</body>
</html>
"""


def render_llms(locale: Locale) -> str:
    ui = ui_copy(locale.code)
    lines = {
        "ja": ["# Automic Vault", "Automic Vault は macOS 上の AI エージェント向けローカルセキュリティレイヤーです。", "シークレットを平文ファイルから離し、承認されたツールだけに渡します。"],
        "de": ["# Automic Vault", "Automic Vault ist eine lokale Sicherheitsschicht für AI-Agents auf macOS.", "Secrets verlassen Klartextdateien und werden nur an genehmigte Tools weitergegeben."],
        "fr": ["# Automic Vault", "Automic Vault est une couche de sécurité locale pour agents IA sur macOS.", "Les secrets quittent les fichiers en clair et ne sont transmis qu'aux outils approuvés."],
        "zh-Hans": ["# Automic Vault", "Automic Vault 是 macOS 上面向 AI 代理的本地安全层。", "密钥不再保存在明文文件中，只会传递给已批准的工具。"],
    }[locale.code]
    return "\n\n".join(lines) + f"\n\n- {ui['website']}: {href('/', locale)}\n- {ui['packages']}: {href('/pkg/', locale)}\n"


def render_i18n_js(locales: list[Locale]) -> str:
    data = [
        {
            "code": locale.code,
            "slug": locale.slug,
            "nativeName": locale.native_name,
            "languages": list(locale.browser_languages),
            "suggestionAria": ui_copy(locale.code)["languageSuggestionAria"],
            "suggestionText": ui_copy(locale.code)["languageSuggestionText"],
            "dismissLabel": ui_copy(locale.code)["dismissLanguageSuggestion"],
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
      banner.setAttribute("aria-label", match.suggestionAria);
      const link = document.createElement("a");
      link.href = localized;
      link.textContent = match.suggestionText;
      const button = document.createElement("button");
      button.type = "button";
      button.setAttribute("aria-label", match.dismissLabel);
      button.textContent = "×";
      button.addEventListener("click", () => {{
        localStorage.setItem(dismissedKey, "1");
        banner.remove();
      }});
      banner.append(link, button);
      document.body.appendChild(banner);
    }})
    .catch(() => {{}});
}})();
"""


def render_home_page(record: dict[str, Any], locale: Locale, locales: list[Locale]) -> str:
    t = record["translations"][locale.code]
    detail = HOME_DETAIL[locale.code]
    ui = ui_copy(locale.code)
    canonical = href("/", locale)
    meta = "".join(f"<span>{html.escape(item)}</span>" for item in detail["meta"])
    brief = "\n".join(
        f"""            <div>
              <span class="tiny-icon{icon_class}" aria-hidden="true"></span>
              <p>{html.escape(text)}</p>
            </div>"""
        for text, icon_class in zip(detail["brief"], ["", " square", " triangle"])
    )
    highlights = "\n".join(
        f"""          <a class="highlight-card {accent}" href="{target}">
            <span>{html.escape(label)}</span>
            <strong>{html.escape(body)}</strong>
          </a>"""
        for (label, body), accent, target in zip(
            detail["highlights"],
            ["accent-hot", "accent-blue", "accent-green", "accent-gold"],
            ["#secrets", "#approval", "#nucleus", locale_path("/av-trace/", locale)],
        )
    )
    approval_prompt = f'{html.escape(ui["approvalPrompt"])} <code>npm publish</code> {html.escape(ui["approvalQuestion"])}'
    story_extras = [
        f'<ul class="story-tags" aria-label="{html.escape(ui["secretBoundaryDetailsAria"], quote=True)}"><li>gh</li><li>aws-cli</li><li>av inject</li><li>secret scanner</li></ul>',
        f'<div class="inline-prompt" aria-label="{html.escape(ui["approvalRequestAria"], quote=True)}"><span>Automic Vault</span><strong>{approval_prompt}</strong><i>{html.escape(ui["deny"])}</i><i>{html.escape(ui["approve"])}</i></div>',
        f'<div class="source-strip" aria-label="{html.escape(ui["packageSourcesAria"], quote=True)}"><span>Homebrew</span><span>npm</span><span>PyPI</span><span>/opt</span></div>',
        "<!-- no extra content -->",
        f'<figure class="app-shot"><img src="/assets/gui-screenshot.webp" alt="{html.escape(ui["screenshotAlt"], quote=True)}" width="1693" height="929"></figure>',
    ]
    stories = "\n".join(
        f"""          <article class="ranked-story feature-section" id="{story_id}">
            <span class="rank">{index}</span>
            <div>
              <p class="story-kicker">{html.escape(kicker)}</p>
              <h3>{html.escape(title)}</h3>
              <p>{html.escape(body)}</p>
              {extra}
            </div>
          </article>"""
        for index, ((kicker, title, body), story_id, extra) in enumerate(
            zip(detail["stories"], ["secrets", "approval", "nucleus", "scanner", "app"], story_extras),
            start=1,
        )
    )
    fit_cards = "\n".join(
        f"""          <article>
            <span>{html.escape(label)}</span>
            <h3>{html.escape(title)}</h3>
            <p>{html.escape(body)}</p>
          </article>"""
        for label, title, body in detail["fit"]
    )
    nav_labels = detail["nav"]
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
{alternate_link_block("/", locales)}
  <link rel="icon" href="/favicon.ico" sizes="16x16 32x32 48x48">
  <link rel="apple-touch-icon" href="/apple-touch-icon.png">
  <link rel="stylesheet" href="/styles.css?v=25">
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
  <div class="scroll-meter" aria-hidden="true"><span></span></div>
  <div class="site-shell" id="top">
    <header class="masthead">
      <a class="brand" href="{locale_path('/', locale)}" aria-label="{html.escape(ui["brandHomeAria"], quote=True)}">
        <img class="brand-mark" src="/assets/icon@2x.webp" alt="" width="54" height="54">
        <span class="brand-type">Automic Vault</span>
      </a>
      <button class="nav-toggle" type="button" aria-expanded="false" aria-label="{html.escape(ui["toggleNavigationAria"], quote=True)}"><span></span><span></span></button>
      <nav class="nav" aria-label="{html.escape(ui["mainNavigationAria"], quote=True)}">
        <a href="#ranked">{html.escape(nav_labels[0])}</a>
        <a href="#secrets">{html.escape(nav_labels[1])}</a>
        <a href="#approval">{html.escape(nav_labels[2])}</a>
        <a href="#nucleus">{html.escape(nav_labels[3])}</a>
        <a href="{locale_path('/pkg/', locale)}">{html.escape(nav_labels[4])}</a>
        <a href="{locale_path('/docs/', locale)}">{html.escape(nav_labels[5])}</a>
        <a href="{locale_path('/download/', locale)}">{html.escape(nav_labels[6])}</a>
        <a href="https://github.com/automic-vault/">GitHub</a>
      </nav>
    </header>
    <main>
      <section class="hero" aria-labelledby="hero-title">
        <div class="hero-meta">{meta}</div>
        <div class="hero-grid">
          <div class="hero-copy">
            <p class="eyebrow">{html.escape(t["kicker"])}</p>
            <h1 id="hero-title">Automic Vault</h1>
            <p class="lede">{html.escape(t["lede"])}</p>
          </div>
          <aside class="hero-brief" aria-label="{html.escape(ui["currentSecurityPostureAria"], quote=True)}">
{brief}
          </aside>
        </div>
        <div class="hero-actions">
          <a class="button primary" href="/Automic Vault.dmg">{html.escape(detail["actions"][0])}</a>
          <a class="button secondary" href="{locale_path('/docs/', locale)}">{html.escape(detail["actions"][1])}</a>
          <a class="button text" href="{locale_path('/secret-scanner-for-ai-agents/', locale)}">{html.escape(detail["actions"][2])}</a>
        </div>
      </section>
      <section class="highlights" aria-labelledby="highlights-title">
        <div class="section-label"><h2 id="highlights-title">{html.escape(ui["highlights"])}</h2><span>{html.escape(ui["securityMap"])}</span></div>
        <div class="highlight-grid">
{highlights}
        </div>
      </section>
      <section class="story-layout" id="ranked" aria-label="{html.escape(ui["rankedFeaturesAria"], quote=True)}">
        <div class="story-main">
          <div class="list-heading"><div><h2>{html.escape(detail["storiesTitle"])}</h2><p>{html.escape(detail["storiesLede"])}</p></div><span>{html.escape(ui["v0Surface"])}</span></div>
{stories}
        </div>
        <aside class="side-rail" aria-label="{html.escape(ui["operationalNotesAria"], quote=True)}">
          <section>
            <h2>{html.escape(ui["runtime"])}</h2>
            <article><span>{html.escape(ui["releaseLabel"])}</span><strong>/opt</strong><p>{html.escape(ui["releaseNote"])}</p></article>
            <article><span>{html.escape(ui["stubsLabel"])}</span><strong>/usr/local/bin</strong><p>{html.escape(ui["stableEntrypoints"])}</p></article>
          </section>
          <section>
            <h2>{html.escape(ui["caseFiles"])}</h2>
            <a class="rail-link" href="{locale_path('/github-cli-token-security-ai-agents/', locale)}"><span>gh</span><strong>{html.escape(ui["caseGithub"])}</strong></a>
            <a class="rail-link" href="{locale_path('/secure-aws-cli-credentials-ai-agents/', locale)}"><span>aws</span><strong>{html.escape(ui["caseAws"])}</strong></a>
            <a class="rail-link" href="{locale_path('/ai-agent-approval-gates/', locale)}"><span>gate</span><strong>{html.escape(ui["caseApproval"])}</strong></a>
          </section>
        </aside>
      </section>
      <section class="compare feature-section" aria-labelledby="compare-title">
        <div class="section-label"><h2 id="compare-title">{html.escape(detail["fitTitle"])}</h2><span>{html.escape(detail["fitKicker"])}</span></div>
        <div class="compare-grid">
{fit_cards}
        </div>
      </section>
      <section class="final-cta" aria-labelledby="final-title">
        <p class="eyebrow">{html.escape(ui["finalKicker"])}</p>
        <h2 id="final-title">{html.escape(detail["final"])}</h2>
        <div><a class="button primary" href="/Automic Vault.dmg">{html.escape(detail["actions"][0])}</a><a class="button secondary" href="https://github.com/automic-vault/automic-vault">{html.escape(ui["viewSource"])}</a></div>
      </section>
      {language_links("/", locale, locales)}
    </main>
    <footer class="site-footer">
      <p>&copy; 2026 Automic Vault.</p>
      <div class="footer-links">
        <a href="{locale_path('/about/', locale)}">{html.escape(ui["about"])}</a>
        <a href="{locale_path('/security/', locale)}">{html.escape(ui["security"])}</a>
        <a href="{locale_path('/privacy/', locale)}">{html.escape(ui["privacy"])}</a>
        <a href="{locale_path('/terms/', locale)}">{html.escape(ui["terms"])}</a>
        <a href="https://x.com/AutomicVault">X</a>
        <a href="https://github.com/automic-vault/">GitHub</a>
      </div>
    </footer>
  </div>
  <script src="/app.js?v=17"></script>
</body>
</html>
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
            expected = render_home_page(record, locale, locales) if record["path"] == "/" else render_page(record, locale, locales)
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
