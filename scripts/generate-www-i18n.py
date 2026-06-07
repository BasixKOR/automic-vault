#!/usr/bin/env python3
from __future__ import annotations

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
    "download": {
        "ja": {"title": "Automic Vault ダウンロード", "description": "macOS 用 Automic Vault を入手し、ローカルの AI エージェント実行を保護します。", "kicker": "ダウンロード", "h1": "macOS 用 Automic Vault をダウンロード", "lede": "ローカルの Homebrew パッケージ、CLI シークレット、AI エージェント操作を保護する macOS セキュリティレイヤーをインストールします。", "sections": [["直接ダウンロード", ".dmg を取得するか、ターミナル用の install.sh スクリプトでインストールできます。"], ["含まれるもの", "ネイティブアプリ、av コマンドラインツール、シークレットスキャナー、Nucleus パッケージ制御が含まれます。"], ["インストール後", "まずシークレットスキャナーを実行し、平文の認証情報を確認して、対応済みのシークレットを保護されたローカル保存に移します。"]]},
        "de": {"title": "Automic Vault herunterladen", "description": "Lade Automic Vault für macOS herunter und schütze lokale AI-Agent-Läufe.", "kicker": "Download", "h1": "Automic Vault für macOS herunterladen", "lede": "Installiere die lokale Sicherheitschicht für Homebrew-Pakete, CLI-Secrets und AI-Agent-Aktionen auf macOS.", "sections": [["Direkter Download", "Lade die .dmg-Datei herunter oder installiere über das install.sh-Skript im Terminal."], ["Was enthalten ist", "Enthalten sind die native App, das av-Kommandozeilenwerkzeug, Secret-Scanner-Workflows und Nucleus-Paketkontrollen."], ["Nach der Installation", "Starte zuerst den Secret Scanner, prüfe Klartext-Credentials und verschiebe unterstützte Secrets in geschützten lokalen Speicher."]]},
        "fr": {"title": "Télécharger Automic Vault", "description": "Téléchargez Automic Vault pour macOS et protégez les exécutions locales d'agents IA.", "kicker": "Téléchargement", "h1": "Télécharger Automic Vault pour macOS", "lede": "Installez la couche de sécurité locale pour les paquets Homebrew, les secrets CLI et les actions d'agents IA sur macOS.", "sections": [["Téléchargement direct", "Téléchargez le .dmg ou installez depuis le terminal avec le script install.sh."], ["Ce qui est inclus", "Le téléchargement inclut l'application native, l'outil en ligne de commande av, les workflows de scanner de secrets et les contrôles de paquets Nucleus."], ["Après l'installation", "Lancez d'abord le scanner de secrets, vérifiez les identifiants en clair et déplacez les secrets pris en charge vers un stockage local protégé."]]},
        "zh-Hans": {"title": "下载 Automic Vault", "description": "获取 macOS 版 Automic Vault，保护本地 AI 代理运行。", "kicker": "下载", "h1": "下载 macOS 版 Automic Vault", "lede": "安装用于保护 macOS 上 Homebrew 软件包、CLI 密钥和 AI 代理操作的本地安全层。", "sections": [["直接下载", "可以下载 .dmg，也可以在终端中使用 install.sh 脚本安装。"], ["包含内容", "下载内容包括原生应用、av 命令行工具、密钥扫描器工作流和 Nucleus 软件包控制。"], ["安装之后", "先运行密钥扫描器，检查明文凭据，并将支持的密钥移入受保护的本地存储。"]]},
    },
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
    "dotenv": {
        "ja": {"title": "AI エージェントに .env を読ませない", "description": ".env の平文シークレットを、承認されたツールへの実行時注入に置き換えます。", "kicker": "dotenv 保護", "h1": "AI エージェントが .env ファイルを読めないようにする", "lede": ".env は便利ですが、エージェントからも読みやすい場所です。Automic Vault は値をローカル保護ストレージへ移し、承認されたコマンドだけに渡します。", "sections": [["前後比較", "以前は .env に API キーやデプロイトークンがあり、通常のファイル確認で漏えいしました。移行後は非シークレット設定だけを残し、av save で保存した値を av inject で必要なプロセスへ渡します。"], ["コマンド例", "av save STRIPE_SECRET_KEY を実行し、.env から値を削除してから av inject -- npm test のように許可されたスクリプトを起動します。"], ["運用メモ", "エージェントに「読まないで」と頼むのではなく、読めるファイルから秘密をなくすことで、ログ、要約、パッチへの流出経路を閉じます。"]]},
        "de": {"title": "Verhindere, dass AI-Agents .env lesen", "description": "Ersetze Klartext-Secrets in .env durch Laufzeitinjektion in genehmigte Tools.", "kicker": "dotenv-Schutz", "h1": "AI-Agents daran hindern, .env-Dateien zu lesen", "lede": ".env ist praktisch, aber für Agents leicht lesbar. Automic Vault verschiebt Werte in geschützten lokalen Speicher und gibt sie nur an genehmigte Befehle weiter.", "sections": [["Vorher / Nachher", "Vorher standen API-Keys und Deploy-Tokens in .env, sodass normales Datei-Inspecting Werte preisgab. Nachher bleiben nur nicht geheime Defaults im Repo; av save speichert den Wert und av inject übergibt ihn an den benötigten Prozess."], ["Befehlsbeispiel", "Führe av save STRIPE_SECRET_KEY aus, entferne die Zeile aus .env und starte den erlaubten Lauf mit av inject -- npm test oder dem konkreten Script."], ["Betrieb", "Verlasse dich nicht darauf, dass der Agent die Datei meidet. Entferne den lesbaren Secret-Pfad, damit Logs, Zusammenfassungen und Patches keine Rohwerte übernehmen."]]},
        "fr": {"title": "Empêcher les agents IA de lire .env", "description": "Remplacez les secrets en clair dans .env par une injection à l'exécution vers les outils approuvés.", "kicker": "protection dotenv", "h1": "Empêcher les agents IA de lire les fichiers .env", "lede": ".env est pratique, mais facile à lire pour un agent. Automic Vault déplace les valeurs vers un stockage local protégé et les transmet seulement aux commandes approuvées.", "sections": [["Avant / après", "Avant, les clés API et jetons de déploiement vivaient dans .env; une inspection normale du fichier pouvait les exposer. Après, le dépôt garde les réglages non secrets, tandis que av save stocke la valeur et av inject la transmet au processus autorisé."], ["Exemple de commande", "Exécutez av save STRIPE_SECRET_KEY, supprimez la ligne de .env, puis lancez le script autorisé avec av inject -- npm test."], ["Note d'exploitation", "Ne demandez pas seulement à l'agent d'éviter le fichier. Supprimez le chemin de lecture afin que journaux, résumés et patches ne capturent pas les valeurs brutes."]]},
        "zh-Hans": {"title": "阻止 AI 代理读取 .env", "description": "把 .env 中的明文密钥替换为面向已批准工具的运行时注入。", "kicker": "dotenv 保护", "h1": "阻止 AI 代理读取 .env 文件", "lede": ".env 很方便，但也很容易被代理读取。Automic Vault 将值移入受保护的本地存储，只交给已批准的命令。", "sections": [["前后对比", "以前 API 密钥和部署令牌写在 .env 中，普通文件检查就可能暴露它们。之后仓库只保留非密钥默认值，用 av save 保存敏感值，并用 av inject 交给需要它的进程。"], ["命令示例", "运行 av save STRIPE_SECRET_KEY，从 .env 删除该行，然后用 av inject -- npm test 或已批准脚本启动。"], ["运维说明", "不要只要求代理不要读取文件。应移除可读密钥路径，避免日志、摘要和补丁捕获原始值。"]]},
    },
    "apiKeys": {
        "ja": {"title": "AI エージェント向け API キー管理", "description": "API キーをモデルコンテキストから外し、承認された CLI にだけ注入します。", "kicker": "API キー管理", "h1": "AI コーディングエージェント向け API キー管理", "lede": "API キーは文字列ではなく権限です。Automic Vault は値をローカルに保存し、承認された実行だけに名前付きキーを渡します。", "sections": [["前後比較", "以前はシェルプロファイル、.npmrc、.netrc、クラウド設定にキーがあり、すべての子プロセスが継承できました。移行後はコマンドごとの機能として扱います。"], ["コマンド例", "av save OPENAI_API_KEY で保存し、広い export を削除してから av inject -- npm test や av inject -- gh release create を実行します。"], ["優先対象", "GitHub、AWS、npm、PyPI、デプロイ CLI から始めます。公開、削除、リリース、クラウド変更ができるキーは最初に保護します。"]]},
        "de": {"title": "API-Key-Management für AI-Agents", "description": "Halte API-Keys aus dem Modellkontext und injiziere sie nur in genehmigte CLIs.", "kicker": "API-Key-Management", "h1": "API-Key-Management für AI-Coding-Agents", "lede": "Ein API-Key ist eine Berechtigung, kein Text für das Modell. Automic Vault speichert Werte lokal und übergibt benannte Keys nur an genehmigte Ausführungen.", "sections": [["Vorher / Nachher", "Vorher lagen Keys in Shell-Profilen, .npmrc, .netrc oder Cloud-Konfigurationen und wurden von jedem Kindprozess geerbt. Nachher wird jeder Key als Fähigkeit pro Befehl behandelt."], ["Befehlsbeispiel", "Speichere den Wert mit av save OPENAI_API_KEY, entferne breite Exports und nutze av inject -- npm test oder av inject -- gh release create für den konkreten Lauf."], ["Prioritäten", "Beginne mit GitHub, AWS, npm, PyPI und Deploy-CLIs. Keys, die veröffentlichen, löschen, releasen oder Cloud-Zustand ändern können, gehören zuerst geschützt."]]},
        "fr": {"title": "Gestion des clés API pour agents IA", "description": "Gardez les clés API hors du contexte modèle et injectez-les seulement dans les CLI approuvées.", "kicker": "gestion des clés API", "h1": "Gestion des clés API pour agents de codage IA", "lede": "Une clé API est une capacité, pas un texte pour le modèle. Automic Vault stocke les valeurs localement et transmet les clés nommées seulement aux exécutions approuvées.", "sections": [["Avant / après", "Avant, les clés vivaient dans les profils shell, .npmrc, .netrc ou les configs cloud et tous les processus enfants pouvaient en hériter. Après, chaque clé devient une capacité par commande."], ["Exemple de commande", "Stockez la valeur avec av save OPENAI_API_KEY, retirez les exports larges, puis utilisez av inject -- npm test ou av inject -- gh release create pour l'exécution exacte."], ["Priorités", "Commencez par GitHub, AWS, npm, PyPI et les CLI de déploiement. Les clés capables de publier, supprimer, releaser ou modifier le cloud doivent être protégées en premier."]]},
        "zh-Hans": {"title": "面向 AI 代理的 API 密钥管理", "description": "让 API 密钥远离模型上下文，只注入到已批准的 CLI。", "kicker": "API 密钥管理", "h1": "面向 AI 编码代理的 API 密钥管理", "lede": "API 密钥是一种权限，不是给模型处理的文本。Automic Vault 在本地保存值，并只把命名密钥交给已批准的执行。", "sections": [["前后对比", "以前密钥位于 shell 配置、.npmrc、.netrc 或云配置中，任何子进程都可能继承。之后每个密钥都作为单次命令能力处理。"], ["命令示例", "使用 av save OPENAI_API_KEY 保存值，移除宽泛 export，然后用 av inject -- npm test 或 av inject -- gh release create 执行具体命令。"], ["优先对象", "先处理 GitHub、AWS、npm、PyPI 和部署 CLI。能发布、删除、发版或改变云状态的密钥应最先保护。"]]},
    },
    "mcp": {
        "ja": {"title": "MCP シークレット管理", "description": "MCP ツールへ認証情報を渡しながら、モデルコンテキストと平文設定からシークレットを遠ざけます。", "kicker": "MCP セキュリティ", "h1": "モデルにシークレットを渡さない MCP シークレット管理", "lede": "MCP はツール利用を簡単にしますが、サーバー設定に生のトークンを置くべきではありません。Automic Vault は起動境界で承認された値を注入します。", "sections": [["前後比較", "以前は MCP JSON や dotenv に GITHUB_TOKEN や DB 認証情報が入り、設定確認で値が見えました。移行後は設定にコマンドだけを残し、値は実行時に渡します。"], ["コマンド例", "av save GITHUB_TOKEN を実行し、MCP 設定から値を削除してから av inject -- ./mcp-server でサーバーを起動します。"], ["承認境界", "読み取り専用ツールは軽く、公開・削除・クラウド変更ができるツールは av contain と承認ゲートに通します。"]]},
        "de": {"title": "MCP-Secret-Management", "description": "Gib MCP-Tools Credentials, ohne Secrets in Modellkontext oder Klartextkonfiguration zu legen.", "kicker": "MCP-Sicherheit", "h1": "MCP-Secret-Management ohne Secrets im Modell", "lede": "MCP vereinfacht Tool-Zugriff, aber rohe Tokens gehören nicht in Serverkonfigurationen. Automic Vault injiziert genehmigte Werte an der lokalen Startgrenze.", "sections": [["Vorher / Nachher", "Vorher standen GITHUB_TOKEN oder Datenbank-Credentials in MCP-JSON oder dotenv-Dateien, sodass Konfigurationsprüfung Werte zeigte. Nachher bleibt nur der Befehl in der Konfiguration; der Wert kommt zur Laufzeit."], ["Befehlsbeispiel", "Führe av save GITHUB_TOKEN aus, entferne den Wert aus der MCP-Konfiguration und starte den Server mit av inject -- ./mcp-server."], ["Freigabegrenze", "Read-only-Tools können leicht laufen; Tools mit Publish-, Delete- oder Cloud-Mutationsrechten gehören durch av contain und Approval Gates."]]},
        "fr": {"title": "Gestion des secrets MCP", "description": "Donnez des identifiants aux outils MCP sans placer les secrets dans le contexte modèle ou la configuration en clair.", "kicker": "sécurité MCP", "h1": "Gestion des secrets MCP sans secrets dans le modèle", "lede": "MCP facilite l'accès aux outils, mais les jetons bruts ne doivent pas vivre dans la configuration serveur. Automic Vault injecte les valeurs approuvées à la limite de lancement locale.", "sections": [["Avant / après", "Avant, GITHUB_TOKEN ou les identifiants de base de données vivaient dans le JSON MCP ou dotenv, donc vérifier la configuration exposait les valeurs. Après, la configuration garde la commande; la valeur arrive à l'exécution."], ["Exemple de commande", "Exécutez av save GITHUB_TOKEN, retirez la valeur de la configuration MCP, puis lancez le serveur avec av inject -- ./mcp-server."], ["Limite d'approbation", "Les outils en lecture seule peuvent rester simples; les outils capables de publier, supprimer ou muter le cloud doivent passer par av contain et des portes d'approbation."]]},
        "zh-Hans": {"title": "MCP 密钥管理", "description": "让 MCP 工具获得凭据，同时避免密钥进入模型上下文或明文配置。", "kicker": "MCP 安全", "h1": "不把密钥交给模型的 MCP 密钥管理", "lede": "MCP 让工具访问更容易，但原始令牌不应写在服务器配置中。Automic Vault 在本地启动边界注入已批准的值。", "sections": [["前后对比", "以前 GITHUB_TOKEN 或数据库凭据写在 MCP JSON 或 dotenv 中，检查配置就会暴露值。之后配置只保留命令，密钥在运行时注入。"], ["命令示例", "运行 av save GITHUB_TOKEN，从 MCP 配置删除值，然后用 av inject -- ./mcp-server 启动服务器。"], ["审批边界", "只读工具可以轻量运行；能发布、删除或修改云资源的工具应通过 av contain 和审批门。"]]},
    },
    "approvalGates": {
        "ja": {"title": "AI エージェント承認ゲート", "description": "AI エージェントが実行する高リスクコマンドを、ツール層で人間が確認します。", "kicker": "コマンド承認", "h1": "AI エージェントが実行するコマンドの承認ゲート", "lede": "意図ではなく、実際に走る実行ファイル、引数、シークレット利用を承認します。Automic Vault はエージェントの下のツール層で停止点を作ります。", "sections": [["前後比較", "以前は高レベルな確認後にエージェントが git push、npm publish、aws s3 rm を同じセッションで実行できました。移行後は危険なコマンドごとに実行直前の承認を要求します。"], ["コマンド例", "エージェント作業を av contain で実行し、公開、削除、デプロイ、認証情報表示、クラウド変更のコマンドに承認ゲートを置きます。"], ["優先ルール", "最初に npm publish、twine upload、gh auth token、git push --force、aws の削除や IAM 変更を対象にします。"]]},
        "de": {"title": "Approval Gates für AI-Agents", "description": "Prüfe riskante Befehle von AI-Agents mit menschlicher Freigabe auf Tool-Ebene.", "kicker": "Befehlsfreigabe", "h1": "Approval Gates für Befehle, die AI-Agents ausführen", "lede": "Gib nicht nur die Absicht frei, sondern das konkrete Executable, die Argumente und den Secret-Zugriff. Automic Vault setzt den Stoppunkt unterhalb des Agents.", "sections": [["Vorher / Nachher", "Vorher konnte ein Agent nach einer allgemeinen Bestätigung git push, npm publish oder aws s3 rm in derselben Sitzung ausführen. Nachher verlangt jeder riskante Befehl Freigabe direkt vor der Ausführung."], ["Befehlsbeispiel", "Führe Agent-Arbeit durch av contain und setze Gates für Publish, Delete, Deploy, Credential-Ausgabe und Cloud-Mutation."], ["Priorität", "Beginne mit npm publish, twine upload, gh auth token, git push --force sowie AWS-Lösch- und IAM-Änderungen."]]},
        "fr": {"title": "Portes d'approbation pour agents IA", "description": "Vérifiez les commandes risquées des agents IA avec une approbation humaine au niveau outil.", "kicker": "approbation de commandes", "h1": "Portes d'approbation pour les commandes exécutées par les agents IA", "lede": "N'approuvez pas seulement l'intention: approuvez l'exécutable, les arguments et l'accès aux secrets. Automic Vault place le point d'arrêt sous l'agent.", "sections": [["Avant / après", "Avant, après une validation générale, l'agent pouvait lancer git push, npm publish ou aws s3 rm dans la même session. Après, chaque commande risquée demande une approbation juste avant exécution."], ["Exemple de commande", "Faites passer le travail de l'agent par av contain et placez des portes sur publication, suppression, déploiement, affichage d'identifiants et mutation cloud."], ["Priorité", "Commencez par npm publish, twine upload, gh auth token, git push --force, ainsi que les suppressions AWS et changements IAM."]]},
        "zh-Hans": {"title": "AI 代理审批门", "description": "在工具层用人工审批检查 AI 代理的高风险命令。", "kicker": "命令审批", "h1": "AI 代理执行命令的审批门", "lede": "不要只批准意图，而要批准实际可执行文件、参数和密钥访问。Automic Vault 在代理下方的工具层设置停止点。", "sections": [["前后对比", "以前一次宽泛确认后，代理可以在同一会话运行 git push、npm publish 或 aws s3 rm。之后每个高风险命令都在执行前请求审批。"], ["命令示例", "通过 av contain 运行代理工作，并为发布、删除、部署、凭据显示和云资源变更设置审批门。"], ["优先规则", "先覆盖 npm publish、twine upload、gh auth token、git push --force，以及 AWS 删除和 IAM 修改。"]]},
    },
}

HOME_DETAIL: dict[str, dict[str, Any]] = {
    "ja": {
        "meta": ["macOS", "Homebrew", "ローカル優先", "2026年6月1日更新"],
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
        "meta": ["macOS", "Homebrew", "lokal zuerst", "aktualisiert am 1. Juni 2026"],
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
        "meta": ["macOS", "Homebrew", "local d'abord", "mis à jour le 1er juin 2026"],
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
        "meta": ["macOS", "Homebrew", "本地优先", "2026 年 6 月 1 日更新"],
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
        "downloadDmg": "Download .dmg",
        "finalKicker": "Free and open source",
        "highlights": "Highlights",
        "home": "Home",
        "installerScript": "Installer script",
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
        "whitePaperEnglish": "Read English paper",
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
        "downloadDmg": ".dmg をダウンロード",
        "finalKicker": "無料のオープンソース",
        "highlights": "ハイライト",
        "home": "ホーム",
        "installerScript": "インストーラースクリプト",
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
        "whitePaperEnglish": "英語版を読む",
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
        "downloadDmg": ".dmg herunterladen",
        "finalKicker": "Kostenlos und Open Source",
        "highlights": "Kernpunkte",
        "home": "Startseite",
        "installerScript": "Installer-Skript",
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
        "whitePaperEnglish": "Englisches White Paper",
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
        "downloadDmg": "Télécharger le .dmg",
        "finalKicker": "Gratuit et open source",
        "highlights": "Points forts",
        "home": "Accueil",
        "installerScript": "Script d'installation",
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
        "whitePaperEnglish": "Livre blanc anglais",
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
        "downloadDmg": "下载 .dmg",
        "finalKicker": "免费开源",
        "highlights": "亮点",
        "home": "首页",
        "installerScript": "安装脚本",
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
        "whitePaperEnglish": "阅读英文版",
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
    "awsCli": {"ja": ("AI エージェント向け AWS CLI 認証情報保護", "AWS 認証情報を Keychain に移し、credential_process ヘルパー経由で root 管理の aws ランチャーだけに渡します。"), "de": ("AWS-CLI-Credentials für AI-Agents schützen", "Verschiebe AWS-Credentials in die Keychain und gib sie über credential_process nur an den root-kontrollierten aws-Launcher weiter."), "fr": ("Sécuriser les identifiants AWS CLI pour agents IA", "Déplacez les identifiants AWS dans le trousseau et transmettez-les via credential_process uniquement au lanceur aws contrôlé par root."), "zh-Hans": ("保护 AI 代理的 AWS CLI 凭据", "将 AWS 凭据移入 Keychain，并通过 credential_process 只交给 root 控制的 aws 启动器。")},
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
        "ja": "Homebrew ツール、CLI シークレット、承認ゲートを Mac 上でローカルに制御し、AI エージェントの権限を明確な境界の中に収めます。",
        "de": "Es kontrolliert Homebrew-Tools, CLI-Secrets und Approval Gates lokal auf dem Mac, damit AI-Agents innerhalb klarer Grenzen arbeiten.",
        "fr": "Il contrôle localement sur Mac les outils Homebrew, les secrets CLI et les portes d'approbation afin que les agents IA restent dans des limites claires.",
        "zh-Hans": "它在 Mac 本地控制 Homebrew 工具、CLI 密钥和审批门，让 AI 代理在清晰边界内运行。",
    }[locale_code]


TITLE_SUFFIX: dict[str, str] = {
    "ja": " | AI エージェントと開発ツールの保護",
    "de": " | Schutz für AI-Agents und Entwickler-Tools",
    "fr": " | Protection des agents IA et outils développeur",
    "zh-Hans": " | AI 代理与开发工具安全",
}


DESCRIPTION_ADDENDUM: dict[str, str] = {
    "ja": "Homebrew ツール、CLI シークレット、承認ゲートを Mac 上でローカルに制御します。",
    "de": "Kontrolliere Homebrew-Tools, CLI-Secrets und Approval Gates lokal auf dem Mac.",
    "fr": "Contrôlez localement sur Mac les outils Homebrew, les secrets CLI et les portes d'approbation.",
    "zh-Hans": "在本地 Mac 上控制 Homebrew 工具、CLI 密钥和审批门。",
}


SUPPORT_SECTIONS: dict[str, list[list[str]]] = {
    "ja": [
        ["ローカル実行境界", "Automic Vault は、エージェントが読めるファイルと、承認されたツールだけが受け取る認証情報を分けます。モデルの指示ではなく、Mac 上の実行経路で制御します。"],
        ["Homebrew と CLI", "多くの開発ツールは Homebrew、npm、PyPI、クラウド CLI から入ります。Vault は、そのツールが作る認証情報ファイルや危険な操作を検出して、必要な場所に承認を置きます。"],
        ["次の手順", "まずスキャナーで平文の露出を確認し、対応済みのシークレットを保護されたローカル保存に移し、Automic Vault を起動したまま新しい hazard 通知を受け取ります。"],
        ["インストール後の変化", "既存の CLI は使い続けられます。ただし、シークレットはエージェントが読める設定ファイルではなく、承認された実行だけが受け取るローカル境界の中に移ります。"],
        ["中央の vault との違い", "1Password や HashiCorp Vault はシークレットの管理元として使えます。Automic Vault は、その値が Mac 上のツールへ渡される瞬間を制御します。"],
        ["関連ページ", "ドキュメント、ダウンロード、シークレットスキャナー、パッケージカタログを確認すると、検出からハードニング、承認、継続監視までの流れが分かります。"],
        ["継続監視", "新しいパッケージ、古いツール、再作成された設定ファイルは、最初の修正後にも危険を戻す可能性があります。Automic Vault は新しい hazard を知らせます。"],
        ["信頼の手がかり", "公開リポジトリ、セキュリティページ、ライセンス、Max Howell の Homebrew 背景を確認できます。ローカルのセキュリティ境界は、検証可能であるべきです。"],
        ["使う場面", "エージェントにリポジトリを任せる前、クラウド CLI を使う前、npm publish や gh release のような権限の強いコマンドを許可する前に使います。"],
    ],
    "de": [
        ["Lokale Laufzeitgrenze", "Automic Vault trennt Dateien, die ein Agent lesen kann, von Credentials, die nur genehmigte Tools erhalten. Die Kontrolle sitzt im Ausführungspfad auf dem Mac, nicht nur in einer Agent-Anweisung."],
        ["Homebrew und CLIs", "Viele Entwicklerwerkzeuge kommen über Homebrew, npm, PyPI und Cloud-CLIs. Vault erkennt Credential-Dateien und riskante Aktionen, die diese Tools hinterlassen, und setzt Freigaben an die passende Stelle."],
        ["Nächste Schritte", "Starte zuerst den Scanner, verschiebe unterstützte Secrets in geschützten lokalen Speicher und lasse Automic Vault laufen, damit neue Hazard-Hinweise sofort sichtbar bleiben."],
        ["Was sich nach der Installation ändert", "Die bekannten CLIs bleiben nutzbar. Der Unterschied ist, dass Secrets nicht mehr als einfache Konfigurationsdateien herumliegen, sondern nur an die genehmigte Ausführung übergeben werden, die sie wirklich braucht."],
        ["Zusammenspiel mit zentralen Vaults", "1Password, HashiCorp Vault und Cloud-Secret-Systeme können weiter die Quelle der Wahrheit bleiben. Automic Vault schützt den lokalen Moment, in dem ein Mac-Tool diese Werte verwenden will."],
        ["Weiterführende Seiten", "Dokumentation, Download, Secret Scanner und Paketkatalog zeigen den Weg von Erkennung zu Härtung, Approval Gates und laufender Überwachung der Entwickler-Maschine."],
        ["Laufende Überwachung", "Neue Pakete, veraltete Tools und neu erzeugte Konfigurationsdateien können nach der ersten Bereinigung wieder Risiken schaffen. Automic Vault bleibt aktiv und meldet frische Hazards."],
        ["Vertrauenshinweise", "Öffentliches Repository, Sicherheitsseite, Lizenz und der Homebrew-Hintergrund von Max Howell sind verlinkt. Eine lokale Sicherheitsgrenze sollte überprüfbar sein."],
        ["Wann einsetzen", "Nutze Automic Vault vor Agent-Läufen in Repositories, vor Cloud-CLI-Arbeit und bevor Befehle wie npm publish, gh release oder Infrastrukturänderungen echte Berechtigungen ausgeben."],
    ],
    "fr": [
        ["Limite d'exécution locale", "Automic Vault sépare les fichiers lisibles par un agent des identifiants transmis uniquement aux outils approuvés. Le contrôle vit dans le chemin d'exécution sur Mac, pas seulement dans une consigne d'agent."],
        ["Homebrew et CLI", "Beaucoup d'outils développeur arrivent par Homebrew, npm, PyPI et les CLI cloud. Vault détecte les fichiers d'identifiants et les actions risquées laissés par ces outils, puis place l'approbation au bon endroit."],
        ["Étapes suivantes", "Lancez d'abord le scanner, déplacez les secrets pris en charge vers un stockage local protégé et gardez Automic Vault actif pour recevoir les nouveaux avis de danger."],
        ["Ce qui change après installation", "Les CLI habituelles continuent de fonctionner. La différence est que les secrets ne restent pas dans de simples fichiers de configuration; ils sont transmis seulement à l'exécution approuvée qui en a besoin."],
        ["Avec un coffre central", "1Password, HashiCorp Vault et les systèmes cloud peuvent rester la source de vérité. Automic Vault protège le moment local où un outil Mac veut utiliser ces valeurs."],
        ["Pages liées", "Documentation, téléchargement, scanner de secrets et catalogue de paquets montrent le parcours complet: détection, durcissement, approbation et surveillance continue du poste développeur."],
        ["Surveillance continue", "Nouveaux paquets, outils obsolètes et fichiers de configuration recréés peuvent ramener des risques après la première correction. Automic Vault reste actif et signale les nouveaux dangers."],
        ["Signaux de confiance", "Le dépôt public, la page sécurité, la licence et le contexte Homebrew de Max Howell sont liés. Une limite de sécurité locale doit pouvoir être vérifiée."],
        ["Quand l'utiliser", "Utilisez Automic Vault avant les exécutions d'agents dans un dépôt, avant le travail avec les CLI cloud et avant les commandes comme npm publish, gh release ou les mutations d'infrastructure."],
    ],
    "zh-Hans": [
        ["本地运行边界", "Automic Vault 将代理可读取的文件与只提供给已批准工具的凭据分开。控制发生在 Mac 的执行路径上，而不是只依赖代理提示词。"],
        ["Homebrew 与 CLI", "许多开发工具来自 Homebrew、npm、PyPI 和云 CLI。Vault 会检测这些工具留下的凭据文件和高风险操作，并把审批放到实际执行位置。"],
        ["下一步", "先运行扫描器检查明文暴露，将支持的密钥移入受保护的本地存储，并保持 Automic Vault 运行，以便及时收到新的 hazard 通知。"],
        ["安装后的变化", "熟悉的 CLI 仍可继续使用。区别在于密钥不再作为容易读取的配置文件留在磁盘上，而是只交给真正需要它的已批准执行。"],
        ["与中心化 vault 配合", "1Password、HashiCorp Vault 和云端密钥系统仍可作为真实来源。Automic Vault 保护的是本地 Mac 工具使用这些值的那一刻。"],
        ["相关页面", "文档、下载、密钥扫描器和软件包目录展示完整流程：发现暴露、加固工具、加入审批门，并持续监控开发机器。"],
        ["持续监控", "新的软件包、过时工具和重新生成的配置文件可能在首次修复后重新带来风险。Automic Vault 会保持运行并提示新的 hazard。"],
        ["可信线索", "公开仓库、安全页面、许可证以及 Max Howell 的 Homebrew 背景都可查看。本地安全边界应该能够被验证。"],
        ["何时使用", "在让代理处理仓库之前、使用云 CLI 之前，以及允许 npm publish、gh release 或基础设施变更等高权限命令之前使用 Automic Vault。"],
    ],
}

LANDING_HOME: dict[str, dict[str, Any]] = {
    "ja": {
        "nav": ["パッケージ", "ブログ", "セキュリティ", "ドキュメント", "ダウンロード"],
        "hero_eyebrow": "Homebrew 作者より",
        "hero_h1": "エージェントが動く前に Mac を堅牢化。",
        "hero_lede": "Mac には、エージェントが読める、漏らせる、マルウェアに渡せる平文キーが残りがちです。Automic Vault は重要な層を堅牢化します。暗号化されたシークレット、封印されたツール、本番変更前の承認です。",
        "download_short": "av.dmg をダウンロード",
        "run_scanner": "スキャナーを実行",
        "view_source": "ソースを見る",
        "harden_title": "パッケージ由来の平文シークレットを消す。",
        "harden_body": "Automic Vault は CLI が dotfile に残す認証情報を見つけ、対応済みのシークレットを Keychain に移し、エージェントが値を読まずに使える実行時ヘルパーを残します。",
        "harden_notes": ["いつも通りツールをインストール", "シークレット露出を検出", "平文シークレットは残念ながらオープンソースでよくあります", "av harden がキーを Keychain に移してヘルパーを書き換えます", "credential_process で aws はキーを受け取り、エージェントやマルウェアには露出しません"],
        "radar_title": "既知のツール、見える制御。",
        "radar_body": "Automic Vault はヘルパープロトコル、一時ホーム、Keychain-backed トークン、平文 hazard 検出のパッケージルールを巡回します。",
        "immutable_title": "インストール後のパッケージを不変に。",
        "immutable_body": "Automic Vault は Homebrew、npm、PyPI のツールを封印された root にインストールし、PATH に av shim を置きます。エージェントはツールを実行できますが、ツール自体の変更は承認済みアップデート経由になります。",
        "immutable_note": "エージェントは自分自身を変更できません",
        "dotenv_title": "ワークフローには .env を。エージェントの文脈には入れない。",
        "dotenv_body": "av dotenv は、秘密鍵を Automic Vault Keychain に残したまま、dotenvx 互換の encrypted: 値を書きます。direnv 風の hook は、承認済みキーだけを、それを要求した shell やコマンドへ読み込みます。",
        "detect_title": "Mac が変わり続ける間に、新しいリスクを検出。",
        "detect_body": "Automic Vault.app は、ツールをインストールし、プロジェクトを編集し、うっかり新しいシークレットを露出する間もバックグラウンドで動きます。新しいパッケージ hazard は発生時に通知され、対応済みの平文シークレットは即座に暗号化されます。",
        "features": [
            ["マルチソースパッケージ", "Homebrew、npm、PyPI のツールをインストールして堅牢化。対応元は今後も増えます。"],
            ["Touch ID インストール", "ターミナルを離れずに権限付きインストールとアップデートを承認。"],
            ["自動アップデート", "アプリと堅牢化ルールをバックグラウンドで最新に保ちます。"],
            ["GUI 検索と発見", "パッケージ、ホームページ、バージョン、hazard、セキュリティノートを閲覧。"],
            ["通知", "新しい脆弱性や露出したシークレットが現れたら分かります。"],
        ],
        "field_eyebrow": "05 · フィールドノート",
        "field_title": "正しい境界は、ずっと前から必要だったように感じる。",
        "field_body": "最初に役立った反応は、数値でもローンチ記事でもありませんでした。ローカルのエージェントセキュリティは、すでにツールチェーンの一部であるべきだという感覚でした。",
        "final_eyebrow": "無料のオープンソース",
        "final_a11y": "brew、npm、pip、cargo、pnpm、uv でインストールするツールを保護します。",
        "final_prefix": "ツールを保護",
        "final_you": "あなたが",
        "read_docs": "ドキュメントを読む",
        "browse_packages": "パッケージを見る",
        "footer_state": [["Hazard", "検出"], ["Secrets", "Keychain-backed"], ["Approvals", "Touch ID ready"]],
    },
    "de": {
        "nav": ["Pakete", "Blog", "Sicherheit", "Dokumentation", "Download"],
        "hero_eyebrow": "Vom Homebrew-Erfinder",
        "hero_h1": "Härtet deinen Mac, bevor der Agent läuft.",
        "hero_lede": "Auf deinem Mac liegen Klartext-Keys, die ein Agent lesen, leaken oder Malware übergeben kann. Automic Vault härtet die Schicht, die zählt: verschlüsselte Secrets, versiegelte Tools und Freigabe vor Produktionsänderungen.",
        "download_short": "av.dmg herunterladen",
        "run_scanner": "Scanner starten",
        "view_source": "Quellcode ansehen",
        "harden_title": "Lass Klartext-Secrets aus Paketen verschwinden.",
        "harden_body": "Automic Vault findet Credentials, die CLIs in Dotfiles ablegen, verschiebt unterstützte Secrets in den Keychain und hinterlässt jedem Tool einen Laufzeit-Helper, den der Agent nutzen kann, ohne das Secret zu lesen.",
        "harden_notes": ["Installiere Tools wie immer", "Erkenne Secret-Exposure", "Klartext-Secrets sind leider üblich in Open Source", "av harden verschiebt Keys in den Keychain und schreibt den Helper um", "credential_process gibt aws Keys, ohne sie Agents oder Malware offenzulegen"],
        "radar_title": "Bekannte Tools, sichtbare Kontrollen.",
        "radar_body": "Automic Vault rotiert durch Paketregeln für Helper-Protokolle, temporäre Homes, Keychain-gestützte Tokens und Klartext-Hazard-Erkennung.",
        "immutable_title": "Mache Pakete nach der Installation unveränderlich.",
        "immutable_body": "Automic Vault installiert Tools aus Homebrew, npm oder PyPI in versiegelte Roots und setzt einen av-Shim auf PATH. Agents können das Tool ausführen; Änderungen am Tool selbst laufen über ein genehmigtes Update.",
        "immutable_note": "Agent kann sich nicht selbst ändern",
        "dotenv_title": ".env bleibt im Workflow, nicht im Agent-Kontext.",
        "dotenv_body": "av dotenv schreibt dotenvx-kompatible encrypted:-Werte, während der private Key im Automic-Vault-Keychain bleibt. Direnv-artige Hooks laden nur genehmigte Keys in die Shell oder den Befehl, der sie angefordert hat.",
        "detect_title": "Erkenne neue Risiken, während dein Mac sich weiter verändert.",
        "detect_body": "Automic Vault.app läuft im Hintergrund, während du Tools installierst, Projekte bearbeitest und versehentlich neue Secrets offenlegst. Neue Paket-Hazards werden sofort gemeldet, und unterstützte Klartext-Secrets werden direkt verschlüsselt.",
        "features": [
            ["Multi-Source-Pakete", "Installiere und härte Tools aus Homebrew, npm und PyPI; weitere Quellen folgen."],
            ["Touch-ID-Installationen", "Genehmige privilegierte Installationen und Updates, ohne das Terminal zu verlassen."],
            ["Auto-Updates", "Halte App und Paketregeln im Hintergrund aktuell."],
            ["GUI-Suche und Discovery", "Durchsuche Pakete, Homepages, Versionen, Hazards und Security Notes."],
            ["Benachrichtigungen", "Erfahre, wenn neue Schwachstellen oder offengelegte Secrets erscheinen."],
        ],
        "field_eyebrow": "05 · Feldnotiz",
        "field_title": "Die richtige Grenze fühlt sich überfällig an.",
        "field_body": "Das erste nützliche Signal war keine Metrik und kein Launch-Post. Es war das Gefühl, dass lokale Agent-Sicherheit längst Teil der Toolchain sein sollte.",
        "final_eyebrow": "Kostenlos und Open Source",
        "final_a11y": "Sichere die Tools, die du mit brew, npm, pip, cargo, pnpm und uv installierst.",
        "final_prefix": "Sichere die Tools",
        "final_you": "die du",
        "read_docs": "Doku lesen",
        "browse_packages": "Pakete ansehen",
        "footer_state": [["Hazards", "erkannt"], ["Secrets", "Keychain-backed"], ["Approvals", "Touch ID ready"]],
    },
    "fr": {
        "nav": ["Paquets", "Blog", "Sécurité", "Docs", "Télécharger"],
        "hero_eyebrow": "Par le créateur de Homebrew",
        "hero_h1": "Durcissez votre Mac avant que l’agent s’exécute.",
        "hero_lede": "Votre Mac contient des clés en clair qu’un agent peut lire, divulguer ou transmettre à un malware. Automic Vault durcit la couche qui compte : secrets chiffrés, outils scellés et approbation avant les changements en production.",
        "download_short": "Télécharger av.dmg",
        "run_scanner": "Lancer le scanner",
        "view_source": "Voir le code",
        "harden_title": "Faites disparaître les secrets en clair des paquets.",
        "harden_body": "Automic Vault trouve les identifiants que les CLI laissent dans les dotfiles, déplace les secrets pris en charge dans le trousseau et laisse à chaque outil un helper d’exécution utilisable par l’agent sans lire le secret.",
        "harden_notes": ["Installez les outils comme d’habitude", "Détectez l’exposition des secrets", "Les secrets en clair sont malheureusement courants dans l’open source", "av harden déplace les clés dans le trousseau et réécrit le helper", "credential_process donne les clés à aws sans les exposer aux agents ou aux malwares"],
        "radar_title": "Outils connus, contrôles visibles.",
        "radar_body": "Automic Vault parcourt les règles de paquets pour les protocoles helper, homes temporaires, jetons adossés au trousseau et détection de dangers en clair.",
        "immutable_title": "Rendez les paquets immuables après installation.",
        "immutable_body": "Automic Vault installe les outils Homebrew, npm ou PyPI dans des racines scellées et place un shim av sur PATH. Les agents peuvent lancer l’outil; modifier l’outil lui-même passe par une mise à jour approuvée.",
        "immutable_note": "l’agent ne peut pas se modifier lui-même",
        "dotenv_title": "Gardez .env dans le workflow, pas dans le contexte de l’agent.",
        "dotenv_body": "av dotenv écrit des valeurs encrypted: compatibles dotenvx pendant que la clé privée reste dans le trousseau Automic Vault. Des hooks façon direnv chargent seulement les clés approuvées dans le shell ou la commande qui les a demandées.",
        "detect_title": "Détectez les nouveaux risques pendant que votre Mac change.",
        "detect_body": "Automic Vault.app tourne en arrière-plan quand vous installez des outils, modifiez des projets et exposez accidentellement de nouveaux secrets. Les nouveaux dangers de paquets sont signalés dès leur apparition, et les secrets en clair pris en charge sont chiffrés immédiatement.",
        "features": [
            ["Paquets multi-sources", "Installez et durcissez des outils Homebrew, npm et PyPI, avec d’autres sources à venir."],
            ["Installations Touch ID", "Approuvez les installations et mises à jour privilégiées sans quitter le terminal."],
            ["Mises à jour automatiques", "Gardez l’app et les règles de durcissement à jour en arrière-plan."],
            ["Recherche et découverte GUI", "Parcourez paquets, homepages, versions, dangers et notes de sécurité."],
            ["Notifications", "Soyez averti quand de nouvelles vulnérabilités ou des secrets exposés apparaissent."],
        ],
        "field_eyebrow": "05 · Note de terrain",
        "field_title": "La bonne limite semble déjà en retard.",
        "field_body": "Le premier signal utile n’était pas une métrique ni un billet de lancement. C’était l’impression que la sécurité locale des agents devrait déjà faire partie de la toolchain.",
        "final_eyebrow": "Gratuit et open source",
        "final_a11y": "Sécurisez les outils que vous installez avec brew, npm, pip, cargo, pnpm et uv.",
        "final_prefix": "Sécurisez les outils",
        "final_you": "que vous",
        "read_docs": "Lire la doc",
        "browse_packages": "Voir les paquets",
        "footer_state": [["Dangers", "détectés"], ["Secrets", "Keychain-backed"], ["Approvals", "Touch ID ready"]],
    },
    "zh-Hans": {
        "nav": ["软件包", "博客", "安全", "文档", "下载"],
        "hero_eyebrow": "来自 Homebrew 作者",
        "hero_h1": "在代理运行前加固你的 Mac。",
        "hero_lede": "你的 Mac 上可能有代理能读取、泄露，甚至交给恶意软件的明文密钥。Automic Vault 加固真正关键的一层：加密密钥、封印工具，以及生产变更前的审批。",
        "download_short": "下载 av.dmg",
        "run_scanner": "运行扫描器",
        "view_source": "查看源码",
        "harden_title": "让软件包里的明文密钥消失。",
        "harden_body": "Automic Vault 会发现 CLI 留在 dotfile 中的凭据，将支持的密钥移入 Keychain，并给每个工具留下运行时 helper，让代理无需读取密钥也能使用工具。",
        "harden_notes": ["照常安装工具", "检测密钥暴露", "明文密钥在开源工具中很常见", "av harden 将密钥移入 Keychain 并重写 helper", "credential_process 让 aws 获得密钥，但不暴露给代理或恶意软件"],
        "radar_title": "已知工具，可见控制。",
        "radar_body": "Automic Vault 会轮询软件包规则，覆盖 helper 协议、临时 home、Keychain-backed token 和明文 hazard 检测。",
        "immutable_title": "安装后让软件包不可变。",
        "immutable_body": "Automic Vault 将来自 Homebrew、npm 或 PyPI 的工具安装到封印 root 中，并在 PATH 上放置 av shim。代理可以运行工具；修改工具本身会转到已批准的更新流程。",
        "immutable_note": "代理不能修改自己",
        "dotenv_title": ".env 留在工作流中，不进入代理上下文。",
        "dotenv_body": "av dotenv 写入 dotenvx 兼容的 encrypted: 值，同时私钥留在 Automic Vault Keychain 中。direnv 风格的 hook 只把已批准的密钥加载到请求它们的 shell 或命令里。",
        "detect_title": "在 Mac 持续变化时检测新风险。",
        "detect_body": "Automic Vault.app 会在后台运行：你安装工具、编辑项目、意外暴露新密钥时，它都会继续监控。新的软件包 hazard 会在出现时报告，支持的明文密钥会立即加密。",
        "features": [
            ["多来源软件包", "安装并加固来自 Homebrew、npm 和 PyPI 的工具，更多来源即将支持。"],
            ["Touch ID 安装", "无需离开终端即可审批特权安装和更新。"],
            ["自动更新", "在后台保持应用和加固规则最新。"],
            ["GUI 搜索与发现", "浏览软件包、主页、版本、hazard 和安全说明。"],
            ["通知", "新的漏洞或暴露密钥出现时及时知道。"],
        ],
        "field_eyebrow": "05 · 现场反馈",
        "field_title": "正确的边界早就该存在了。",
        "field_body": "最有用的第一条信号不是指标，也不是发布文章，而是一种感觉：本地代理安全本应已经是工具链的一部分。",
        "final_eyebrow": "免费且开源",
        "final_a11y": "保护你用 brew、npm、pip、cargo、pnpm 和 uv 安装的工具。",
        "final_prefix": "保护你的工具",
        "final_you": "当你",
        "read_docs": "阅读文档",
        "browse_packages": "浏览软件包",
        "footer_state": [["Hazards", "已检测"], ["Secrets", "Keychain-backed"], ["Approvals", "Touch ID ready"]],
    },
}


def normalized_title(title: str, locale: Locale) -> str:
    if locale.code == "en":
        return title
    if len(title) >= 34 or "|" in title:
        return title
    return title + TITLE_SUFFIX[locale.code]


def normalized_description(description: str, locale: Locale) -> str:
    if locale.code == "en" or len(description) >= 105:
        return description
    addendum = DESCRIPTION_ADDENDUM[locale.code]
    if addendum in description:
        return description
    return f"{description} {addendum}"


def expanded_sections(translations: dict[str, Any], locale: Locale) -> list[list[str]]:
    sections = [list(item) for item in translations.get("sections", [])]
    seen = {title for title, _ in sections}
    for title, body in SUPPORT_SECTIONS.get(locale.code, []):
        if len(sections) >= 11:
            break
        if title in seen:
            continue
        sections.append([title, body])
        seen.add(title)
    return sections


def render_page(record: dict[str, Any], locale: Locale, locales: list[Locale]) -> str:
    path = record["path"]
    t = record["translations"][locale.code]
    ui = ui_copy(locale.code)
    root = rel_root(path, locale)
    canonical = href(path, locale)
    page_title = normalized_title(t["title"], locale)
    page_description = normalized_description(t["description"], locale)
    sections = "\n".join(
        f"""      <section class="i18n-section">
        <h2>{html.escape(title)}</h2>
        <p>{html.escape(body)}</p>
      </section>"""
        for title, body in expanded_sections(t, locale)
    )
    if path == "/download/":
        hero_actions = f"""        <div class="hero-actions">
          <a class="button primary" href="/Automic Vault.dmg">{html.escape(ui["downloadDmg"])}</a>
          <a class="button secondary" href="/install.sh">{html.escape(ui["installerScript"])}</a>
        </div>"""
    elif path == "/security/whitepaper/":
        hero_actions = f"""        <div class="hero-actions">
          <a class="button primary" href="/security/whitepaper/">{html.escape(ui["whitePaperEnglish"])}</a>
          <a class="button secondary" href="{locale_path('/security/', locale)}">{html.escape(ui["security"])}</a>
        </div>"""
    else:
        hero_actions = f"""        <div class="hero-actions">
          <a class="button primary" href="{locale_path('/download/', locale)}">{html.escape(ui["download"])}</a>
          <a class="button secondary" href="{locale_path('/docs/', locale)}">{html.escape(ui["docs"])}</a>
        </div>"""
    return f"""<!DOCTYPE html>
<html lang="{html.escape(locale.html_lang)}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{html.escape(page_title)}</title>
  <meta name="description" content="{html.escape(page_description, quote=True)}">
  <meta name="robots" content="index,follow">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="Automic Vault">
  <meta property="og:title" content="{html.escape(page_title, quote=True)}">
  <meta property="og:description" content="{html.escape(page_description, quote=True)}">
  <meta property="og:url" content="{html.escape(canonical, quote=True)}">
  <meta property="og:image" content="{SITE_ORIGIN}/preview.jpg">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="{html.escape(page_title, quote=True)}">
  <meta name="twitter:description" content="{html.escape(page_description, quote=True)}">
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
    "name": {json.dumps(page_title, ensure_ascii=False)},
    "description": {json.dumps(page_description, ensure_ascii=False)},
    "inLanguage": "{locale.html_lang}",
    "isPartOf": {{"@type": "WebSite", "name": "Automic Vault", "url": "{SITE_ORIGIN}/"}}
  }}
  </script>
</head>
<body>
  <div class="site-shell i18n-page">
    <header class="masthead">
      <a class="brand" href="{locale_path('/', locale)}" aria-label="{html.escape(ui["brandHomeAria"], quote=True)}">
        <img class="brand-mark" src="/assets/icon@2x.webp" alt="Automic Vault" width="54" height="54">
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
{hero_actions}
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
        "ja": ["# Automic Vault", "brew install したツールを安全に。", "Automic Vault は、AI エージェントが使う前に、Homebrew ツール、CLI シークレット、コマンド承認ゲートを Mac 上でローカルに保護します。"],
        "de": ["# Automic Vault", "Sichere die Tools, die du mit brew install installierst.", "Automic Vault sichert Homebrew-Tools, CLI-Secrets und Command-Approval-Gates lokal auf deinem Mac, bevor AI-Agents sie verwenden."],
        "fr": ["# Automic Vault", "Sécurisez les outils que vous installez avec brew.", "Automic Vault sécurise localement sur votre Mac les outils Homebrew, les secrets CLI et les portes d'approbation des commandes avant que les agents IA les utilisent."],
        "zh-Hans": ["# Automic Vault", "保护你用 brew install 装上的工具。", "Automic Vault 在 AI 代理使用之前，在你的 Mac 本地保护 Homebrew 工具、CLI 密钥和命令审批门。"],
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
    ui = ui_copy(locale.code)
    landing = LANDING_HOME[locale.code]
    canonical = href("/", locale)
    nav = landing["nav"]
    harden_notes = "\n".join(
        f"            <p>{html.escape(note)}</p>"
        for note in landing["harden_notes"]
    )
    feature_items = "\n".join(
        f"            <li><strong>{html.escape(title)}.</strong> {html.escape(body)}</li>"
        for title, body in landing["features"]
    )
    footer_state = "\n        ".join(
        f"<span><strong>{html.escape(title)}</strong> {html.escape(body)}</span>"
        for title, body in landing["footer_state"]
    )
    language_nav = language_links("/", locale, locales)
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
  <meta property="og:image" content="{SITE_ORIGIN}/assets/app-hazard-journey.webp">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="{html.escape(t["title"], quote=True)}">
  <meta name="twitter:description" content="{html.escape(t["description"], quote=True)}">
  <meta name="twitter:image" content="{SITE_ORIGIN}/assets/app-hazard-journey.webp">
  <link rel="canonical" href="{html.escape(canonical, quote=True)}">
{alternate_link_block("/", locales)}
  <link rel="alternate" type="text/plain" title="llms.txt" href="/llms.txt">
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600;700;800&amp;family=Geist+Mono:wght@400;500;600;700&amp;display=swap" rel="stylesheet">
  <link rel="icon" href="/favicon.ico" sizes="16x16 32x32 48x48">
  <link rel="apple-touch-icon" href="/apple-touch-icon.png">
  <link rel="stylesheet" href="/styles.css?v=75">
  <script type="application/ld+json">
  {{
    "@context": "https://schema.org",
    "@type": "WebPage",
    "url": "{canonical}",
    "name": {json.dumps(t["title"], ensure_ascii=False)},
    "headline": {json.dumps(landing["hero_h1"], ensure_ascii=False)},
    "description": {json.dumps(t["description"], ensure_ascii=False)},
    "inLanguage": "{locale.html_lang}",
    "image": "{SITE_ORIGIN}/assets/app-hazard-journey.webp",
    "isPartOf": {{"@type": "WebSite", "name": "Automic Vault", "url": "{SITE_ORIGIN}/"}}
  }}
  </script>
</head>
<body>
  <div class="scroll-meter" aria-hidden="true"><span></span></div>
  <header class="masthead" id="top">
    <a class="brand" href="{locale_path('/', locale)}" aria-label="{html.escape(ui["brandHomeAria"], quote=True)}">
      <img class="brand-mark" src="/assets/icon@2x.webp" alt="Automic Vault" width="54" height="54">
      <span class="brand-type">Automic Vault</span>
    </a>
    <button class="nav-toggle" type="button" aria-expanded="false" aria-label="{html.escape(ui["toggleNavigationAria"], quote=True)}"><span></span><span></span></button>
    <nav class="nav" aria-label="{html.escape(ui["mainNavigationAria"], quote=True)}">
      <a href="{locale_path('/pkg/', locale)}">{html.escape(nav[0])}</a>
      <a href="{locale_path('/blog/', locale)}">{html.escape(nav[1])}</a>
      <a href="{locale_path('/security/', locale)}">{html.escape(nav[2])}</a>
      <a href="{locale_path('/docs/', locale)}">{html.escape(nav[3])}</a>
      <a href="{locale_path('/download/', locale)}">{html.escape(nav[4])}</a>
      <a href="https://github.com/automic-vault/">GitHub</a>
    </nav>
  </header>

  <div class="site-shell">
    <main class="landing-main">
      <section class="poster-hero" aria-labelledby="hero-title">
        <div class="poster-hero-copy">
          <p class="eyebrow">{html.escape(landing["hero_eyebrow"])}</p>
          <h1 id="hero-title">{html.escape(landing["hero_h1"])}</h1>
          <p class="poster-lede">{html.escape(landing["hero_lede"])}</p>
        </div>

        <div class="poster-hero-foot">
          <div class="hero-actions">
            <a class="button primary" href="/Automic Vault.dmg">{html.escape(landing["download_short"])}</a>
            <a class="button secondary" href="{locale_path('/secret-scanner-for-ai-agents/', locale)}">{html.escape(landing["run_scanner"])}</a>
            <a class="button text" href="https://github.com/automic-vault/automic-vault">{html.escape(landing["view_source"])}</a>
          </div>
        </div>
      </section>

      <section class="feature-section harden-section" id="harden" aria-labelledby="harden-title">
        <div class="feature-copy">
          <p class="eyebrow">01 &middot; Harden</p>
          <h2 id="harden-title">{html.escape(landing["harden_title"])}</h2>
          <p>{html.escape(landing["harden_body"])}</p>
        </div>

        <div class="terminal-story" aria-label="Terminal examples for package hardening">
          <aside class="terminal-notes" aria-label="What this terminal demonstrates">
{harden_notes}
          </aside>
          <figure class="terminal-card terminal-card-green">
            <figcaption>Harden package-owned secrets</figcaption>
            <pre><code><span class="prompt">$</span> brew install awscli
installed: /opt/homebrew/Cellar/awscli

<span class="prompt">$</span> av scan
<span class="terminal-danger">plaintext secret exposure</span>: ~/.aws/credentials
reason agent-readable cloud keys

<span class="prompt">$</span> cat ~/.aws/credentials
[default]
aws_access_key_id=AKIA<span class="secret-blur">IOSFODNN7EXAMPLE</span>
aws_secret_access_key=wJalr<span class="secret-blur">XUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY</span>

<span class="prompt">$</span> av harden awscli
saved AWS keys in Keychain
rewrote credential_process helper
removed plaintext credentials
made install immutable

<span class="prompt">$</span> cat ~/.aws/credentials
[default]
credential_process = /usr/local/bin/av credential-helper aws
</code></pre>
          </figure>
        </div>

        <div class="package-hardening-radar secured-radar" aria-label="Packages Automic Vault hardens">
          <div class="radar-copy">
            <span>Package hardening feed</span>
            <strong>{html.escape(landing["radar_title"])}</strong>
            <p>{html.escape(landing["radar_body"])}</p>
          </div>
          <div class="radar-feed" data-secured-feed>
            <a class="feed-row accent-green" href="/pkg/brew/gh/" aria-label="gh"><span>gh</span><p>GitHub tokens saved in Keychain and injected only for gh commands</p></a>
            <a class="feed-row accent-green" href="/pkg/brew/awscli/" aria-label="awscli"><span>awscli</span><p>AWS keys moved from ~/.aws/credentials to credential_process</p></a>
            <a class="feed-row accent-hot" href="/pkg/brew/curl/" aria-label="curl"><span>curl</span><p>netrc and curlrc credentials detected as hazards</p></a>
            <a class="feed-row accent-gold" href="/pkg/brew/git/" aria-label="git"><span>git</span><p>plaintext credential-store files flagged before agent runs</p></a>
            <a class="feed-row accent-blue" href="/pkg/brew/node/" aria-label="npm"><span>npm</span><p>registry tokens mounted through a temporary npm config</p></a>
          </div>
        </div>
      </section>

      <section class="feature-section immutable-section" id="immutable" aria-labelledby="immutable-title">
        <div class="feature-copy">
          <p class="eyebrow">02 &middot; Harden</p>
          <h2 id="immutable-title">{html.escape(landing["immutable_title"])}</h2>
          <p>{html.escape(landing["immutable_body"])}</p>
        </div>
        <div class="terminal-story" aria-label="Terminal example for immutable package roots">
          <aside class="terminal-notes immutable-terminal-notes" aria-label="What this terminal demonstrates">
            <p>{html.escape(landing["immutable_note"])}</p>
          </aside>
          <figure class="terminal-card terminal-card-blue" aria-label="Agent cannot modify itself">
            <figcaption aria-hidden="true"></figcaption>
            <pre><code><span class="prompt">$</span> av install codex
awaiting Touch ID for root escalation… approved
prefix: /opt/codex
shim: /usr/local/bin/codex -> /opt/codex/bin/codex

<span class="prompt">$</span> codex exec "patch $(which codex)"
permission denied</code></pre>
          </figure>
        </div>
      </section>

      <section class="feature-section dotenv-section" id="dotenv" aria-labelledby="dotenv-title">
        <div class="feature-copy">
          <p class="eyebrow">03 &middot; Securing .env files</p>
          <h2 id="dotenv-title">{html.escape(landing["dotenv_title"])}</h2>
          <p>{html.escape(landing["dotenv_body"])}</p>
        </div>
        <div class="dotenv-terminal-layout" aria-label="Terminal examples for encrypted dotenv files">
          <figure class="terminal-card terminal-card-green dotenv-terminal-single">
            <figcaption>Encrypted .env with direnv-style loading</figcaption>
            <pre><code><span class="prompt">$</span> cd ~/work/billing-api

<span class="prompt">$</span> cat .env
APP_ENV=development
LOG_LEVEL=debug
PUBLIC_SITE_URL=https://billing.example
STRIPE_SECRET_KEY=sk_live_<span class="secret-blur">4eC39HqLyjWDarjtT1zdp7dc</span>
OPENAI_API_KEY=sk-proj-<span class="secret-blur">BJN9zqY2Q9p7xWm8kL3n</span>
DATABASE_URL=postgres://billing:<span class="secret-blur">s3cr3t</span>@db.internal/app

<span class="prompt">$</span> av dotenv init
found existing .env
stored DOTENV_PRIVATE_KEY in Keychain

<span class="prompt">$</span> av dotenv encrypt
encrypted .env

<span class="prompt">$</span> cat .env
DOTENV_PUBLIC_KEY="BP9x..."
APP_ENV=development
LOG_LEVEL=debug
PUBLIC_SITE_URL=https://billing.example
STRIPE_SECRET_KEY="<span class="terminal-encrypted">encrypted:</span>BO/8An..."
OPENAI_API_KEY="<span class="terminal-encrypted">encrypted:</span>BJN9z..."
DATABASE_URL="<span class="terminal-encrypted">encrypted:</span>BI4p3..."
<span class="terminal-comment"># ^^ dotenvx compatible</span>

<span class="prompt">$</span> echo 'eval "$(av dotenv hook zsh)"' >> ~/.zshrc

<span class="terminal-session-divider">----- NEW TERMINAL SESSION -----</span>

<span class="prompt">$</span> cd ~/work/billing-api
av: human approval required… approved
av: +STRIPE_SECRET_KEY +OPENAI_API_KEY +DATABASE_URL

<span class="prompt">$</span> npm run dev
<span class="terminal-comment"># keys exported into this shell only</span></code></pre>
          </figure>
        </div>
      </section>

      <section class="feature-section app-section detect-section" id="detect" aria-labelledby="detect-title">
        <div class="feature-copy">
          <p class="eyebrow">04 &middot; Detect</p>
          <h2 id="detect-title">{html.escape(landing["detect_title"])}</h2>
          <p>{html.escape(landing["detect_body"])}</p>
          <ul class="app-feature-list">
{feature_items}
          </ul>
        </div>
        <figure class="app-visual">
          <img src="/assets/app-hazard-journey.webp" alt="Automic Vault app" width="1672" height="941" loading="lazy" decoding="async">
        </figure>
      </section>

      <section class="feature-section endorsement-section" id="endorsements" aria-labelledby="endorsements-title">
        <div class="feature-copy">
          <p class="eyebrow">{html.escape(landing["field_eyebrow"])}</p>
          <h2 id="endorsements-title">{html.escape(landing["field_title"])}</h2>
          <p>{html.escape(landing["field_body"])}</p>
        </div>
        <a class="tweet-endorsement" href="https://x.com/Hiraweb3/status/2059662670006038857?s=20" target="_blank" rel="noopener noreferrer">
          <div class="tweet-endorsement-person">
            <img src="/assets/hira-avatar.webp" alt="Hira avatar" width="160" height="160" loading="lazy" decoding="async">
            <cite class="tweet-endorsement-author"><span>Hira</span><span class="tweet-endorsement-handle">@Hiraweb3</span><span class="tweet-endorsement-source" aria-hidden="true">𝕏</span></cite>
          </div>
          <blockquote><p>we needed this yesterday but i'll take it now</p></blockquote>
        </a>
      </section>

      <section class="closing-cta" aria-labelledby="final-title">
        <p class="eyebrow">{html.escape(landing["final_eyebrow"])}</p>
        <h2 id="final-title" aria-label="{html.escape(landing["final_a11y"], quote=True)}">
          {html.escape(landing["final_prefix"])}
          <span class="tool-command-line">{html.escape(landing["final_you"])} <span class="tool-flip" aria-hidden="true"><span class="tool-flip-word" data-tool-flip>brew install</span></span></span>
        </h2>
        <div>
          <a class="button primary" href="/Automic Vault.dmg">{html.escape(ui["download"])}</a>
          <a class="button secondary" href="{locale_path('/docs/', locale)}">{html.escape(landing["read_docs"])}</a>
          <a class="button text" href="{locale_path('/pkg/', locale)}">{html.escape(landing["browse_packages"])}</a>
        </div>
      </section>
    </main>

    <footer class="site-footer">
      <div class="footer-brand">
        <img src="/assets/icon@2x.webp" alt="" width="54" height="54" loading="lazy" decoding="async">
        <p>&copy; 2026 Automic Vault.</p>
      </div>
      <div class="footer-state" aria-label="Automic Vault local boundary">
        {footer_state}
      </div>
      <nav class="footer-links" aria-label="Footer navigation">
        <a href="{locale_path('/about/', locale)}">{html.escape(ui["about"])}</a>
        <a href="{locale_path('/security/', locale)}">{html.escape(ui["security"])}</a>
        <a href="{locale_path('/blog/', locale)}">Blog</a>
        <a href="{locale_path('/privacy/', locale)}">{html.escape(ui["privacy"])}</a>
        <a href="{locale_path('/terms/', locale)}">{html.escape(ui["terms"])}</a>
        <a href="https://x.com/AutomicVault">X</a>
        <a href="https://github.com/automic-vault/">GitHub</a>
      </nav>
    </footer>
  </div>

  <script src="/app.js?v=21"></script>
  {language_nav}
  <script src="/i18n.js" defer></script>
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
        ("https://www.automicvault.com/blog/", "2026-06-04"),
        ("https://www.automicvault.com/blog/agent-pack/", "2026-06-04"),
        ("https://www.automicvault.com/blog/unix-plus-plus/", "2026-06-04"),
        ("https://www.automicvault.com/blog/agentic-toolkit/", "2026-06-02"),
        ("https://www.automicvault.com/blog/prevent-nx-console-vscode-compromise/", "2026-05-21"),
        ("https://www.automicvault.com/blog/prevent-github-vscode-extension-breach/", "2026-05-21"),
        ("https://www.automicvault.com/blog/prevent-durabletask-pypi-compromise/", "2026-05-20"),
        ("https://www.automicvault.com/blog/prevent-tanstack-npm-compromise/", "2026-05-15"),
        ("https://www.automicvault.com/blog/prevent-node-ipc-npm-backdoor/", "2026-05-15"),
        ("https://www.automicvault.com/blog/prevent-bitwarden-cli-npm-compromise/", "2026-04-23"),
        ("https://www.automicvault.com/blog/prevent-litellm-pypi-compromise/", "2026-03-25"),
        ("https://www.automicvault.com/llms.txt", "2026-06-01"),
        ("https://www.automicvault.com/llms-full.txt", "2026-06-01"),
        ("https://www.automicvault.com/pricing.md", "2026-06-01"),
        ("https://www.automicvault.com/.well-known/security.txt", "2026-06-01"),
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
