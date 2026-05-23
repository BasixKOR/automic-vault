#!/usr/bin/env python3
import argparse
import datetime as dt
import hashlib
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
FORMULA_URL = "https://formulae.brew.sh/api/formula.json"
CACHE_DIR = Path("cache")
ECOSYSTEM = "brew.sh"
META_KEY = "__pkgdb_meta__"
PAYLOAD_KEY = "__pkgdb_payload__"
CHECK_INTERVAL_SECONDS = 24 * 60 * 60
DEFAULT_TIMEOUT = 60
USER_AGENT = "nucleus/0.1"
OUTPUT_PATH = Path("data/pkg-page-enrichment.json")


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
            self.red = "\033[31m"
            self.green = "\033[32m"
            self.blue = "\033[34m"
            self.reset = "\033[0m"
            self.step = "◆"
            self.ok = "✓"
            self.error = "✗"
        else:
            self.bold = self.red = self.green = self.blue = self.reset = ""
            self.step = ">"
            self.ok = "OK"
            self.error = "ERROR"

    def log(self, message: str = "") -> None:
        if not self.json_mode:
            print(message, file=sys.stderr)

    def step_log(self, message: str) -> None:
        self.log(f"{self.blue}{self.step}{self.reset} {self.bold}{message}{self.reset}")

    def ok_log(self, message: str) -> None:
        self.log(f"  {self.green}{self.ok}{self.reset} {message}")

    def error_log(self, message: str) -> None:
        self.log(f"{self.red}{self.error}{self.reset} {message}")


def ensure_cwd() -> Path:
    scripts_dir = Path(__file__).resolve().parent
    root = scripts_dir.parent
    os.chdir(root)
    return root


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def cache_path_for(url: str, ecosystem: str = ECOSYSTEM) -> Path:
    digest = hashlib.sha256(url.encode("utf-8")).hexdigest()
    return CACHE_DIR / ecosystem / f"{digest}.json"


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_cached_json(url: str, ecosystem: str = ECOSYSTEM) -> tuple[Any, dict[str, Any]]:
    data = read_json(cache_path_for(url, ecosystem))
    if isinstance(data, dict) and META_KEY in data and PAYLOAD_KEY in data:
        return data.get(PAYLOAD_KEY), data.get(META_KEY) or {}
    return data, {}


def write_cache(path: Path, payload: Any, etag: str | None, checked_at: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(
            {
                META_KEY: {"etag": etag, "checked_at": checked_at},
                PAYLOAD_KEY: payload,
            },
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )


def fetch_json(url: str, force_refresh: bool = False) -> Any:
    path = cache_path_for(url)
    payload = None
    meta: dict[str, Any] = {}
    if path.exists():
        payload, meta = read_cached_json(url)

    checked_at = meta.get("checked_at")
    now = int(time.time())
    if (
        not force_refresh
        and isinstance(checked_at, int)
        and now - checked_at < CHECK_INTERVAL_SECONDS
    ):
        return payload

    headers = {"Accept": "application/json", "User-Agent": USER_AGENT}
    etag = meta.get("etag")
    if etag:
        headers["If-None-Match"] = etag
    request = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=DEFAULT_TIMEOUT) as response:
            payload = json.loads(response.read())
            write_cache(path, payload, response.headers.get("etag"), now)
            return payload
    except urllib.error.HTTPError as err:
        if err.code == 304 and payload is not None:
            write_cache(path, payload, etag, now)
            return payload
        if payload is not None:
            print(f"Using cached data for {url}: {err}", file=sys.stderr)
            return payload
        raise
    except urllib.error.URLError as err:
        if payload is not None:
            print(f"Using cached data for {url}: {err}", file=sys.stderr)
            return payload
        raise


def normalize_list(value: Any) -> list[str]:
    if isinstance(value, str):
        return [value] if value else []
    if not isinstance(value, list):
        return []
    result = []
    for item in value:
        if isinstance(item, str) and item:
            result.append(item)
        elif isinstance(item, dict):
            name = item.get("name") or item.get("formula")
            if isinstance(name, str) and name:
                result.append(name)
    return sorted(set(result))


def normalize_license(value: Any) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        return " AND ".join(str(item) for item in value if item)
    if not isinstance(value, dict):
        return ""
    if "any_of" in value:
        return " OR ".join(normalize_license(item) for item in value.get("any_of") or [] if normalize_license(item))
    if "all_of" in value:
        return " AND ".join(normalize_license(item) for item in value.get("all_of") or [] if normalize_license(item))
    if "with" in value:
        base = normalize_license(value.get("with"))
        exception = normalize_license(value.get("exception"))
        return f"{base} WITH {exception}" if base and exception else base
    return ""


def stable_version(formula: dict[str, Any]) -> str:
    versions = formula.get("versions") or {}
    value = versions.get("stable") if isinstance(versions, dict) else None
    return value if isinstance(value, str) else ""


def source_archive(formula: dict[str, Any]) -> str:
    urls = formula.get("urls") or {}
    stable = urls.get("stable") if isinstance(urls, dict) else None
    if not isinstance(stable, dict):
        return ""
    url = stable.get("url")
    return url if isinstance(url, str) else ""


def bottle_metadata(formula: dict[str, Any]) -> dict[str, Any]:
    bottle = formula.get("bottle") or {}
    stable = bottle.get("stable") if isinstance(bottle, dict) else None
    if not isinstance(stable, dict):
        return {"available": False}
    files = stable.get("files") or {}
    platforms = sorted(str(key) for key in files if key) if isinstance(files, dict) else []
    result: dict[str, Any] = {"available": bool(platforms)}
    root_url = stable.get("root_url")
    if isinstance(root_url, str) and root_url:
        result["rootUrl"] = root_url
    if platforms:
        result["platforms"] = platforms
    return result


def install_behavior(formula: dict[str, Any]) -> dict[str, Any]:
    behavior: dict[str, Any] = {
        "postInstallDefined": bool(formula.get("post_install_defined")),
        "service": "declared" if formula.get("service") else None,
    }
    caveats = formula.get("caveats")
    if isinstance(caveats, str) and caveats.strip():
        behavior["caveats"] = re.sub(r"\s+", " ", caveats).strip()
    return behavior


def executable_index(db: dict[str, Any]) -> dict[str, list[str]]:
    result: dict[str, set[str]] = {}
    entries = db.get("entries") or {}
    if not isinstance(entries, dict):
        return {}
    for executable, provider_key in entries.items():
        if not isinstance(executable, str) or not executable:
            continue
        if not isinstance(provider_key, str) or not provider_key:
            continue
        if ":" in provider_key:
            provider, name = provider_key.split(":", 1)
            if provider == "formula":
                provider = "brew"
        else:
            provider, name = "brew", provider_key
        if provider == "brew" and name:
            result.setdefault(name, set()).add(executable)
    return {name: sorted(executables) for name, executables in result.items()}


def executable_records(name: str, executables: dict[str, list[str]]) -> list[dict[str, str]]:
    return [
        {
            "name": executable,
            "kind": "cli",
            "exposure": "global executable",
        }
        for executable in executables.get(name, [])
    ]


def formula_enrichment(formula: dict[str, Any], executables: dict[str, list[str]]) -> tuple[str, dict[str, Any]] | None:
    name = formula.get("name")
    if not isinstance(name, str) or not name:
        return None
    if formula.get("disabled"):
        return None

    entry: dict[str, Any] = {
        "package": {
            "provider": "brew",
            "name": name,
            "packageManager": "Homebrew",
            "packageManagerUrl": f"https://formulae.brew.sh/formula/{name}",
        },
        "version": stable_version(formula),
        "homepage": formula.get("homepage") if isinstance(formula.get("homepage"), str) else "",
        "license": normalize_license(formula.get("license")),
        "sourceArchive": source_archive(formula),
        "dependencies": normalize_list(formula.get("dependencies")),
        "buildDependencies": normalize_list(formula.get("build_dependencies")),
        "usesFromMacos": normalize_list(formula.get("uses_from_macos")),
        "bottle": bottle_metadata(formula),
        "installBehavior": install_behavior(formula),
        "executables": executable_records(name, executables),
    }
    return f"brew:{name}", prune(entry)


def prune(value: Any) -> Any:
    if isinstance(value, dict):
        pruned = {}
        for key, child in value.items():
            child = prune(child)
            if child is not None:
                pruned[key] = child
        return pruned or None
    if isinstance(value, list):
        pruned = []
        for child in value:
            child = prune(child)
            if child is not None:
                pruned.append(child)
        return pruned or None
    if value in ("", [], None):
        return None
    return value


def build_enrichment(formulae: list[Any], db: dict[str, Any]) -> dict[str, Any]:
    executables = executable_index(db)
    packages: dict[str, Any] = {}
    for formula in formulae:
        if not isinstance(formula, dict):
            continue
        enriched = formula_enrichment(formula, executables)
        if enriched is None:
            continue
        key, entry = enriched
        packages[key] = entry
    return {
        "schema": SCHEMA_VERSION,
        "generated_at": utc_now(),
        "packages": dict(sorted(packages.items())),
    }


def expected_enrichment(force_refresh: bool = False) -> dict[str, Any]:
    formulae = fetch_json(FORMULA_URL, force_refresh=force_refresh)
    if not isinstance(formulae, list):
        raise ValueError("Homebrew formula API payload must be a list")
    db = read_json(Path("data/db.json"))
    if not isinstance(db, dict):
        raise ValueError("data/db.json must contain an object")
    return build_enrichment(formulae, db)


def check_current(path: Path, terminal: Terminal) -> int:
    if not path.exists():
        terminal.error_log(f"Missing {path}. Run scripts/generate-pkg-page-enrichment.py.")
        return 1
    try:
        current = read_json(path)
        expected = expected_enrichment()
    except (OSError, ValueError, json.JSONDecodeError) as err:
        terminal.error_log(f"Unable to validate {path}: {err}")
        return 1
    failures = []
    if current.get("schema") != SCHEMA_VERSION:
        failures.append(f"schema is {current.get('schema')!r}, expected {SCHEMA_VERSION}")
    if not current.get("generated_at"):
        failures.append("missing generated_at")
    if current.get("packages") != expected.get("packages"):
        failures.append("package enrichment does not match current Homebrew formula data and data/db.json")
    if failures:
        terminal.error_log("Package page enrichment is stale.")
        for failure in failures:
            terminal.log(f"  - {failure}")
        terminal.log("Run scripts/generate-pkg-page-enrichment.py and regenerate package pages.")
        return 1
    terminal.ok_log(f"Package page enrichment is current ({len(current.get('packages') or {}):,} packages)")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate Homebrew package-page enrichment data.")
    parser.add_argument("--check", action="store_true", help="Validate that the output already matches current inputs.")
    parser.add_argument("--refresh", action="store_true", help="Refresh cached Homebrew formula API data.")
    parser.add_argument("--output", default=str(OUTPUT_PATH), help=f"Path to write or validate. Defaults to {OUTPUT_PATH}.")
    parser.add_argument("--json", action="store_true", help="Print machine-readable status and disable terminal styling.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    ensure_cwd()
    terminal = Terminal(json_mode=args.json)
    output_path = Path(args.output)
    if args.check:
        return check_current(output_path, terminal)
    try:
        terminal.step_log("Building Homebrew package-page enrichment")
        enrichment = expected_enrichment(force_refresh=args.refresh)
    except (OSError, ValueError, urllib.error.URLError, json.JSONDecodeError) as err:
        terminal.error_log(f"Failed to build enrichment: {err}")
        return 1
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(enrichment, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    terminal.ok_log(f"Wrote {len(enrichment.get('packages') or {}):,} package enrichments to {output_path}")
    if args.json:
        print(json.dumps({"ok": True, "output": str(output_path), "package_count": len(enrichment.get("packages") or {})}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
