#!/usr/bin/env python3
import argparse
import gzip
import hashlib
import io
import json
import os
import re
import subprocess
import sqlite3
import tarfile
import tempfile
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
OUTPUT_PATH = Path("data/pkg-manager-indexes.json")
CACHE_DIR = Path("cache/pkg-manager-indexes")
DEFAULT_TIMEOUT = 90
USER_AGENT = "AutomicVaultPkgManagerIndexes/1.0"


MANAGER_DEFINITIONS: dict[str, dict[str, Any]] = {
    "macports": {
        "display_name": "MacPorts",
        "platform": "macos",
        "command_template": "sudo port install {id}",
        "source_label": "MacPorts ports tree",
        "urls": ["https://api.github.com/repos/macports/macports-ports/git/trees/master?recursive=1"],
    },
    "nix": {
        "display_name": "Nix",
        "platform": "linux",
        "command_template": "nix profile install nixpkgs#{id}",
        "source_label": "nixpkgs package tree",
        "urls": ["https://api.github.com/repos/NixOS/nixpkgs/git/trees/master?recursive=1"],
    },
    "apt": {
        "display_name": "apt",
        "platform": "linux",
        "command_template": "sudo apt install {id}",
        "source_label": "Ubuntu 24.04 LTS package indexes",
        "urls": [
            "https://archive.ubuntu.com/ubuntu/dists/noble/main/binary-amd64/Packages.gz",
            "https://archive.ubuntu.com/ubuntu/dists/noble/universe/binary-amd64/Packages.gz",
            "https://archive.ubuntu.com/ubuntu/dists/noble/multiverse/binary-amd64/Packages.gz",
            "https://archive.ubuntu.com/ubuntu/dists/noble/restricted/binary-amd64/Packages.gz",
        ],
    },
    "dnf": {
        "display_name": "dnf",
        "platform": "linux",
        "command_template": "sudo dnf install {id}",
        "source_label": "Fedora Rawhide package metadata",
        "urls": [
            "https://dl.fedoraproject.org/pub/fedora/linux/development/rawhide/Everything/x86_64/os/repodata/repomd.xml"
        ],
    },
    "pacman": {
        "display_name": "pacman",
        "platform": "linux",
        "command_template": "sudo pacman -S {id}",
        "source_label": "Arch Linux sync databases",
        "urls": [
            "https://geo.mirror.pkgbuild.com/core/os/x86_64/core.db.tar.gz",
            "https://geo.mirror.pkgbuild.com/extra/os/x86_64/extra.db.tar.gz",
        ],
    },
    "apk": {
        "display_name": "apk",
        "platform": "linux",
        "command_template": "sudo apk add {id}",
        "source_label": "Alpine Linux edge package indexes",
        "urls": [
            "https://dl-cdn.alpinelinux.org/alpine/edge/main/x86_64/APKINDEX.tar.gz",
            "https://dl-cdn.alpinelinux.org/alpine/edge/community/x86_64/APKINDEX.tar.gz",
            "https://dl-cdn.alpinelinux.org/alpine/edge/testing/x86_64/APKINDEX.tar.gz",
        ],
    },
    "zypper": {
        "display_name": "zypper",
        "platform": "linux",
        "command_template": "sudo zypper install {id}",
        "source_label": "openSUSE Tumbleweed package metadata",
        "urls": ["https://download.opensuse.org/tumbleweed/repo/oss/repodata/repomd.xml"],
    },
    "winget": {
        "display_name": "winget",
        "platform": "windows",
        "command_template": "winget install --id {id} -e",
        "source_label": "Windows Package Manager source index",
        "urls": ["https://cdn.winget.microsoft.com/cache/source.msix"],
    },
    "chocolatey": {
        "display_name": "Chocolatey",
        "platform": "windows",
        "command_template": "choco install {id}",
        "source_label": "Chocolatey community package catalog",
        "urls": ["https://community.chocolatey.org/api/v2/Packages()?$filter=IsLatestVersion&$select=Id&$top=1000"],
        "max_pages": 80,
    },
    "scoop": {
        "display_name": "Scoop",
        "platform": "windows",
        "command_template": "scoop install {id}",
        "source_label": "Scoop official bucket manifest trees",
        "urls": [
            "https://api.github.com/repos/ScoopInstaller/Main/git/trees/master?recursive=1",
            "https://api.github.com/repos/ScoopInstaller/Extras/git/trees/master?recursive=1",
            "https://api.github.com/repos/ScoopInstaller/Versions/git/trees/master?recursive=1",
        ],
    },
}


PACKAGE_ALIAS_MATCHES: dict[str, dict[str, list[str]]] = {
    "node": {
        "apt": ["nodejs"],
        "dnf": ["nodejs"],
        "pacman": ["nodejs"],
        "apk": ["nodejs"],
        "zypper": ["nodejs"],
        "nix": ["nodejs"],
    },
    "postgresql": {
        "apt": ["postgresql-client"],
        "apk": ["postgresql-client"],
    },
}


class Terminal:
    def __init__(self, json_mode: bool = False):
        self.json_mode = json_mode

    def log(self, message: str) -> None:
        if not self.json_mode:
            print(message)

    def ok(self, message: str) -> None:
        self.log(f"OK {message}")

    def error(self, message: str) -> None:
        self.log(f"ERROR {message}")


def ensure_cwd() -> Path:
    scripts_dir = Path(__file__).resolve().parent
    root = scripts_dir.parent
    os.chdir(root)
    return root


def utc_now() -> str:
    import datetime as dt

    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def stable_hash_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def stable_hash(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def normalize_match(value: str) -> str:
    value = value.lower().strip().removeprefix("@")
    value = re.sub(r"[@_/+.]+", "-", value)
    value = re.sub(r"[^a-z0-9-]+", "-", value)
    value = re.sub(r"-+", "-", value)
    return value.strip("-")


def cache_path_for_url(url: str) -> Path:
    parsed = urllib.parse.urlparse(url)
    suffix = Path(parsed.path).suffix or ".data"
    if parsed.path.endswith(".tar.gz"):
        suffix = ".tar.gz"
    digest = hashlib.sha256(url.encode("utf-8")).hexdigest()[:20]
    host = re.sub(r"[^A-Za-z0-9_.-]+", "-", parsed.netloc)
    return CACHE_DIR / f"{host}-{digest}{suffix}"


def fetch_bytes(url: str, *, force_refresh: bool = False) -> bytes:
    path = cache_path_for_url(url)
    if path.exists() and not force_refresh:
        return path.read_bytes()
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "*/*"})
    with urllib.request.urlopen(request, timeout=DEFAULT_TIMEOUT) as response:
        data = response.read()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return data


def maybe_gzip_decompress(data: bytes, url: str = "") -> bytes:
    if url.endswith(".gz") or data.startswith(b"\x1f\x8b"):
        return gzip.decompress(data)
    return data


def maybe_decompress(data: bytes, url: str = "") -> bytes:
    data = maybe_gzip_decompress(data, url)
    if url.endswith(".zst"):
        result = subprocess.run(["zstd", "-dc"], input=data, capture_output=True, check=True)
        return result.stdout
    return data


def package_stanzas(text: str) -> list[dict[str, str]]:
    result: list[dict[str, str]] = []
    current: dict[str, str] = {}
    last_key = ""
    for line in text.replace("\r\n", "\n").split("\n"):
        if not line.strip():
            if current:
                result.append(current)
                current = {}
                last_key = ""
            continue
        if line.startswith((" ", "\t")) and last_key:
            current[last_key] = f"{current[last_key]}\n{line.strip()}"
            continue
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        last_key = key.strip()
        current[last_key] = value.strip()
    if current:
        result.append(current)
    return result


def record(package_id: str, *, match_names: list[str] | None = None, source_url: str = "", source_name: str = "") -> dict[str, Any]:
    names = [package_id]
    names.extend(match_names or [])
    normalized = sorted({normalize_match(name) for name in names if normalize_match(name)})
    return {
        "id": package_id,
        "match_names": normalized,
        "source_name": source_name or package_id,
        "source_url": source_url,
    }


def dedupe_records(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for item in records:
        package_id = str(item.get("id") or "").strip()
        if not package_id:
            continue
        existing = result.get(package_id)
        if existing is None:
            result[package_id] = {
                "id": package_id,
                "match_names": sorted(set(item.get("match_names") or [])),
                "source_name": item.get("source_name") or package_id,
                "source_url": item.get("source_url") or "",
            }
            continue
        existing["match_names"] = sorted(set(existing.get("match_names") or []) | set(item.get("match_names") or []))
    return [result[key] for key in sorted(result)]


def parse_macports_ports_json(data: bytes, source_url: str) -> list[dict[str, Any]]:
    payload = json.loads(data.decode("utf-8"))
    if isinstance(payload, dict):
        items = payload.get("ports") or payload.get("results") or payload.get("packages") or []
    else:
        items = payload
    records = []
    for item in items:
        if not isinstance(item, dict):
            continue
        name = str(item.get("name") or item.get("portdir") or "").strip()
        if name:
            records.append(record(name, source_url=source_url))
    return dedupe_records(records)


def parse_nix_packages_json(data: bytes, source_url: str) -> list[dict[str, Any]]:
    payload = json.loads(data.decode("utf-8"))
    items = payload.get("packages") if isinstance(payload, dict) else payload
    records = []
    if isinstance(items, dict):
        iterable = items.items()
    elif isinstance(items, list):
        iterable = ((str(item.get("attr") or item.get("attribute") or item.get("pname") or item.get("name") or ""), item) for item in items if isinstance(item, dict))
    else:
        iterable = []
    for attr, item in iterable:
        if not isinstance(item, dict):
            continue
        attr = str(item.get("attr") or item.get("attribute") or attr).strip()
        pname = str(item.get("pname") or item.get("name") or "").strip()
        if not attr:
            continue
        records.append(record(attr, match_names=[pname] if pname else [], source_url=source_url, source_name=pname or attr))
    return dedupe_records(records)


def parse_debian_packages(data: bytes, source_url: str) -> list[dict[str, Any]]:
    text = maybe_gzip_decompress(data, source_url).decode("utf-8", errors="replace")
    records = []
    for stanza in package_stanzas(text):
        name = stanza.get("Package", "").strip()
        if name:
            records.append(record(name, source_url=source_url))
    return dedupe_records(records)


def parse_pacman_db(data: bytes, source_url: str) -> list[dict[str, Any]]:
    records = []
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:*") as tar:
        for member in tar:
            if not member.isfile() or not member.name.endswith("/desc"):
                continue
            handle = tar.extractfile(member)
            if handle is None:
                continue
            text = handle.read().decode("utf-8", errors="replace")
            match = re.search(r"^%NAME%\n([^\n]+)", text, flags=re.MULTILINE)
            if match:
                records.append(record(match.group(1).strip(), source_url=source_url))
    return dedupe_records(records)


def parse_apk_index(data: bytes, source_url: str) -> list[dict[str, Any]]:
    records = []
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:*") as tar:
        for member in tar:
            if member.name.endswith("APKINDEX") and member.isfile():
                handle = tar.extractfile(member)
                if handle is None:
                    continue
                text = handle.read().decode("utf-8", errors="replace")
                for stanza in text.split("\n\n"):
                    name = ""
                    for line in stanza.splitlines():
                        if line.startswith("P:"):
                            name = line[2:].strip()
                            break
                    if name:
                        records.append(record(name, source_url=source_url))
    return dedupe_records(records)


def parse_repomd_primary_location(data: bytes, base_url: str) -> str:
    root = ET.fromstring(data)
    namespace = {"repo": "http://linux.duke.edu/metadata/repo"}
    for item in root.findall("repo:data", namespace):
        if item.get("type") != "primary":
            continue
        location = item.find("repo:location", namespace)
        href = location.get("href") if location is not None else ""
        if href:
            repo_root = base_url.rsplit("/repodata/", 1)[0] + "/"
            return urllib.parse.urljoin(repo_root, href)
    raise ValueError("repomd.xml did not contain primary metadata")


def parse_rpm_primary(data: bytes, source_url: str) -> list[dict[str, Any]]:
    text = maybe_decompress(data, source_url)
    records = []
    for _event, element in ET.iterparse(io.BytesIO(text), events=("end",)):
        if not element.tag.endswith("package"):
            continue
        if element.get("type") not in {"rpm", None}:
            element.clear()
            continue
        name = ""
        for child in element:
            if child.tag.endswith("name") and child.text:
                name = child.text.strip()
                break
        if name:
            records.append(record(name, source_url=source_url))
        element.clear()
    return dedupe_records(records)


def parse_github_tree(data: bytes, source_url: str, manager: str) -> list[dict[str, Any]]:
    payload = json.loads(data.decode("utf-8"))
    tree = payload.get("tree") if isinstance(payload, dict) else []
    records = []
    for item in tree:
        if not isinstance(item, dict) or item.get("type") != "blob":
            continue
        path = str(item.get("path") or "")
        if manager == "macports":
            if path.endswith("/Portfile") and path.count("/") >= 2:
                port_name = path.rsplit("/", 2)[-2]
                records.append(record(port_name, source_url=source_url, source_name=path))
        elif manager == "nix":
            match = re.fullmatch(r"pkgs/by-name/[^/]+/([^/]+)/package\.nix", path)
            if match:
                attr = match.group(1)
                records.append(record(attr, source_url=source_url, source_name=path))
        elif manager == "winget":
            match = re.search(r"/([^/]+)\.installer\.ya?ml$", path, flags=re.IGNORECASE)
            if match:
                package_id = match.group(1)
                records.append(record(package_id, source_url=source_url, source_name=path))
        elif manager == "scoop":
            if re.fullmatch(r"(?:bucket/)?[^/]+\.json", path):
                bucket = "main" if "ScoopInstaller/Main" in source_url else "extras" if "ScoopInstaller/Extras" in source_url else "versions"
                package_id = f"{bucket}/{Path(path).stem}"
                records.append(record(package_id, match_names=[Path(path).stem], source_url=source_url, source_name=path))
    return dedupe_records(records)


def parse_winget_source_msix(data: bytes, source_url: str) -> list[dict[str, Any]]:
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        db_data = archive.read("Public/index.db")
    with tempfile.NamedTemporaryFile(suffix=".db") as database:
        database.write(db_data)
        database.flush()
        connection = sqlite3.connect(database.name)
        try:
            records = []
            rows = connection.execute(
                """
                SELECT DISTINCT ids.id, names.name, monikers.moniker
                FROM manifest
                JOIN ids ON ids.rowid = manifest.id
                JOIN names ON names.rowid = manifest.name
                JOIN monikers ON monikers.rowid = manifest.moniker
                """
            )
            for package_id, name, moniker in rows:
                package_id = str(package_id or "").strip()
                if not package_id:
                    continue
                match_names = [str(name or ""), str(moniker or "")]
                records.append(record(package_id, match_names=match_names, source_url=source_url, source_name=package_id))
            return dedupe_records(records)
        finally:
            connection.close()


def parse_chocolatey_atom(data: bytes, source_url: str) -> tuple[list[dict[str, Any]], str]:
    root = ET.fromstring(data)
    atom = {"atom": "http://www.w3.org/2005/Atom", "d": "http://schemas.microsoft.com/ado/2007/08/dataservices"}
    records = []
    for entry in root.findall("atom:entry", atom):
        content_id = entry.find(".//d:Id", atom)
        title = entry.find("atom:title", atom)
        package_id = ""
        if content_id is not None and content_id.text:
            package_id = content_id.text.strip()
        elif title is not None and title.text:
            package_id = title.text.strip()
        if package_id:
            records.append(record(package_id, source_url=source_url))
    next_url = ""
    next_node = root.find("atom:link[@rel='next']", atom)
    if next_node is not None:
        next_url = next_node.get("href") or ""
    return dedupe_records(records), next_url


def build_records_for_manager(manager: str, urls: list[str], *, force_refresh: bool = False, max_pages: int | None = None) -> tuple[list[dict[str, Any]], list[dict[str, str]]]:
    records: list[dict[str, Any]] = []
    source_fingerprints: list[dict[str, str]] = []
    for url in urls:
        data = fetch_bytes(url, force_refresh=force_refresh)
        source_fingerprints.append({"url": url, "sha256": stable_hash_bytes(data)})
        if manager == "macports":
            records.extend(parse_github_tree(data, url, manager))
        elif manager == "nix":
            records.extend(parse_github_tree(data, url, manager))
        elif manager == "apt":
            records.extend(parse_debian_packages(data, url))
        elif manager == "pacman":
            records.extend(parse_pacman_db(data, url))
        elif manager == "apk":
            records.extend(parse_apk_index(data, url))
        elif manager in {"dnf", "zypper"}:
            primary_url = parse_repomd_primary_location(data, url)
            primary_data = fetch_bytes(primary_url, force_refresh=force_refresh)
            source_fingerprints.append({"url": primary_url, "sha256": stable_hash_bytes(primary_data)})
            records.extend(parse_rpm_primary(primary_data, primary_url))
        elif manager == "winget":
            records.extend(parse_winget_source_msix(data, url))
        elif manager == "scoop":
            records.extend(parse_github_tree(data, url, manager))
        elif manager == "chocolatey":
            next_url = url
            seen_pages = set()
            page_count = 0
            while next_url and next_url not in seen_pages:
                if max_pages is not None and page_count >= max_pages:
                    break
                page_count += 1
                seen_pages.add(next_url)
                page_data = data if next_url == url else fetch_bytes(next_url, force_refresh=force_refresh)
                if next_url != url:
                    source_fingerprints.append({"url": next_url, "sha256": stable_hash_bytes(page_data)})
                page_records, next_url = parse_chocolatey_atom(page_data, next_url)
                records.extend(page_records)
    return dedupe_records(records), source_fingerprints


def apply_alias_matches(managers: dict[str, Any]) -> None:
    for local_name, by_manager in PACKAGE_ALIAS_MATCHES.items():
        for manager, package_ids in by_manager.items():
            packages = managers.get(manager, {}).get("packages")
            if not isinstance(packages, dict):
                continue
            for package_id in package_ids:
                item = packages.get(package_id)
                if not isinstance(item, dict):
                    continue
                names = set(item.get("match_names") or [])
                names.add(normalize_match(local_name))
                item["match_names"] = sorted(names)


def build_manager_indexes(*, force_refresh: bool = False) -> dict[str, Any]:
    managers: dict[str, Any] = {}
    for manager, definition in MANAGER_DEFINITIONS.items():
        records, source_fingerprints = build_records_for_manager(
            manager,
            definition["urls"],
            force_refresh=force_refresh,
            max_pages=definition.get("max_pages"),
        )
        managers[manager] = {
            "display_name": definition["display_name"],
            "platform": definition["platform"],
            "command_template": definition["command_template"],
            "source_label": definition["source_label"],
            "sources": source_fingerprints,
            "packages": {item["id"]: item for item in records},
        }
    apply_alias_matches(managers)
    artifact = {
        "schema": SCHEMA_VERSION,
        "generated_at": utc_now(),
        "description": "Compact package-manager database indexes used to emit source-backed install commands on generated package pages.",
        "definition_hash": stable_hash(MANAGER_DEFINITIONS),
        "alias_hash": stable_hash(PACKAGE_ALIAS_MATCHES),
        "managers": managers,
    }
    failures = validate_artifact(artifact)
    if failures:
        raise ValueError("; ".join(failures[:12]))
    return artifact


def validate_artifact(artifact: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if artifact.get("schema") != SCHEMA_VERSION:
        failures.append(f"schema is {artifact.get('schema')!r}, expected {SCHEMA_VERSION}")
    managers = artifact.get("managers")
    if not isinstance(managers, dict):
        return failures + ["managers must be an object"]
    missing = sorted(set(MANAGER_DEFINITIONS) - set(managers))
    if missing:
        failures.append(f"missing manager indexes: {', '.join(missing)}")
    for manager, definition in managers.items():
        if not isinstance(definition, dict):
            failures.append(f"{manager}: manager entry must be an object")
            continue
        if not definition.get("display_name"):
            failures.append(f"{manager}: missing display_name")
        if definition.get("platform") not in {"macos", "linux", "windows", "portable"}:
            failures.append(f"{manager}: invalid platform")
        if "{id}" not in str(definition.get("command_template") or ""):
            failures.append(f"{manager}: command_template must include {{id}}")
        packages = definition.get("packages")
        if not isinstance(packages, dict) or not packages:
            failures.append(f"{manager}: packages must be a non-empty object")
            continue
        for package_id, item in list(packages.items())[:200]:
            if not isinstance(item, dict):
                failures.append(f"{manager}:{package_id}: package record must be an object")
                continue
            if item.get("id") != package_id:
                failures.append(f"{manager}:{package_id}: id must match package key")
            match_names = item.get("match_names")
            if not isinstance(match_names, list) or not match_names:
                failures.append(f"{manager}:{package_id}: match_names must be non-empty")
            if not item.get("source_url"):
                failures.append(f"{manager}:{package_id}: missing source_url")
    if artifact.get("definition_hash") != stable_hash(MANAGER_DEFINITIONS):
        failures.append("definition_hash does not match generator definitions")
    if artifact.get("alias_hash") != stable_hash(PACKAGE_ALIAS_MATCHES):
        failures.append("alias_hash does not match generator aliases")
    return failures


def check_current(path: Path, terminal: Terminal) -> int:
    if not path.exists():
        terminal.error(f"Missing {path}. Run scripts/generate-pkg-manager-indexes.py.")
        return 1
    try:
        artifact = read_json(path)
        failures = validate_artifact(artifact)
    except (OSError, ValueError, json.JSONDecodeError) as err:
        terminal.error(f"Unable to validate {path}: {err}")
        return 1
    if failures:
        terminal.error("Package manager indexes are invalid.")
        for failure in failures[:24]:
            terminal.log(f"  - {failure}")
        return 1
    count = sum(len(manager.get("packages") or {}) for manager in (artifact.get("managers") or {}).values() if isinstance(manager, dict))
    terminal.ok(f"Package manager indexes are valid ({count:,} records)")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate compact source-backed package-manager indexes.")
    parser.add_argument("--check", action="store_true", help="Validate the checked-in artifact without network access.")
    parser.add_argument("--refresh", action="store_true", help="Refresh cached downloaded databases before generating.")
    parser.add_argument("--output", default=str(OUTPUT_PATH), help=f"Path to write or validate. Defaults to {OUTPUT_PATH}.")
    parser.add_argument("--json", action="store_true", help="Print machine-readable status.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    ensure_cwd()
    terminal = Terminal(json_mode=args.json)
    output_path = Path(args.output)
    if args.check:
        return check_current(output_path, terminal)
    try:
        artifact = build_manager_indexes(force_refresh=args.refresh)
    except (OSError, ValueError, urllib.error.URLError, json.JSONDecodeError, ET.ParseError, tarfile.TarError) as err:
        terminal.error(f"Failed to build package manager indexes: {err}")
        return 1
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(artifact, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    count = sum(len(manager.get("packages") or {}) for manager in (artifact.get("managers") or {}).values() if isinstance(manager, dict))
    terminal.ok(f"Wrote {count:,} package manager records to {output_path}")
    if args.json:
        print(json.dumps({"ok": True, "output": str(output_path), "record_count": count}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
