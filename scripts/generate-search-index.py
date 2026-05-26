#!/usr/bin/env python3
import argparse
import datetime as dt
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
SITE_DIR = Path("www")
INDEX_DIR_NAME = "pagefind"
MANIFEST_NAME = ".manifest.json"


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

    def error_log(self, message: str) -> None:
        self.log(f"{self.red}{self.error}{self.reset} {message}")


def ensure_cwd() -> Path:
    scripts_dir = Path(__file__).resolve().parent
    root = scripts_dir.parent
    os.chdir(root)
    return root


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def fmt_int(value: Any) -> str:
    try:
        return f"{int(value):,}"
    except (TypeError, ValueError):
        return str(value)


def source_files(site_dir: Path, index_dir: Path) -> list[Path]:
    files: list[Path] = []
    if site_dir.exists():
        for path in site_dir.rglob("*"):
            if not path.is_file():
                continue
            if index_dir in path.parents:
                continue
            if path.name == ".DS_Store":
                continue
            if path.suffix.lower() in {".html", ".htm"}:
                files.append(path)
    files.append(Path("scripts/generate-search-index.py"))
    i18n_root = Path("data/www-i18n")
    if i18n_root.exists():
        files.extend(path for path in i18n_root.rglob("*.json") if path.is_file())
    i18n_generator = Path("scripts/generate-www-i18n.py")
    if i18n_generator.exists():
        files.append(i18n_generator)
    return sorted(set(files), key=lambda path: path.as_posix())


def source_digest(files: list[Path]) -> tuple[str, int]:
    digest = hashlib.sha256()
    latest = 0
    for path in files:
        stat = path.stat()
        latest = max(latest, stat.st_mtime_ns)
        digest.update(path.as_posix().encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest(), latest


def build_manifest(files: list[Path], pagefind_version: str) -> dict[str, Any]:
    digest, latest = source_digest(files)
    latest_dt = dt.datetime.fromtimestamp(latest / 1_000_000_000, dt.timezone.utc)
    return {
        "schema": SCHEMA_VERSION,
        "generated_at": utc_now(),
        "source_hash": digest,
        "source_file_count": len(files),
        "latest_source_mtime_ns": latest,
        "latest_source_mtime": latest_dt.replace(microsecond=0).isoformat(),
        "pagefind_version": pagefind_version,
    }


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def pagefind_command(pagefind_bin: str) -> list[str]:
    if pagefind_bin:
        return [pagefind_bin]
    if shutil.which("pagefind"):
        return ["pagefind"]
    if shutil.which("npx"):
        return ["npx", "-y", "pagefind"]
    raise RuntimeError("Missing pagefind. Install it or run with npx available.")


def pagefind_version(command: list[str]) -> str:
    try:
        result = subprocess.run(
            command + ["--version"],
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError:
        return "unknown"
    return (result.stdout or result.stderr).strip() or "unknown"


def generate(site_dir: Path, index_dir: Path, pagefind_bin: str, terminal: Terminal) -> dict[str, Any]:
    files = source_files(site_dir, index_dir)
    if not files:
        raise RuntimeError(f"No HTML files found in {site_dir}")
    command = pagefind_command(pagefind_bin)
    version = pagefind_version(command)
    if index_dir.exists():
        shutil.rmtree(index_dir)

    terminal.step_log("Running Pagefind")
    with tempfile.TemporaryDirectory(prefix="pagefind-") as temp_dir:
        temp_root = Path(temp_dir)
        temp_site = temp_root / "site"
        temp_index = temp_root / index_dir.name
        shutil.copytree(
            site_dir,
            temp_site,
            ignore=shutil.ignore_patterns(index_dir.name, ".DS_Store"),
        )
        subprocess.run(
            command + ["--site", "site", "--output-path", index_dir.name],
            check=True,
            cwd=temp_root,
        )
        shutil.copytree(temp_index, index_dir)

    files = source_files(site_dir, index_dir)
    manifest = build_manifest(files, version)
    index_dir.mkdir(parents=True, exist_ok=True)
    (index_dir / MANIFEST_NAME).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return manifest


def check_current(site_dir: Path, index_dir: Path, terminal: Terminal) -> int:
    manifest_path = index_dir / MANIFEST_NAME
    if not manifest_path.exists():
        terminal.error_log(f"Missing {manifest_path}. Run scripts/generate-search-index.py before deploy.")
        return 1
    try:
        manifest = read_json(manifest_path)
    except json.JSONDecodeError as err:
        terminal.error_log(f"Invalid {manifest_path}: {err}")
        return 1

    files = source_files(site_dir, index_dir)
    expected_hash, latest = source_digest(files)
    failures: list[str] = []
    if manifest.get("schema") != SCHEMA_VERSION:
        failures.append(f"schema is {manifest.get('schema')!r}, expected {SCHEMA_VERSION}")
    if manifest.get("source_hash") != expected_hash:
        failures.append("source hash does not match current HTML files")
    if manifest.get("latest_source_mtime_ns", 0) < latest:
        failures.append("search index is older than a source HTML file")
    if int(manifest.get("source_file_count") or 0) != len(files):
        failures.append(f"manifest source count is {manifest.get('source_file_count')}, but found {len(files)}")
    for required in ("pagefind.js", "pagefind-ui.js", "pagefind-entry.json"):
        if not (index_dir / required).exists():
            failures.append(f"missing Pagefind asset: {index_dir / required}")

    if failures:
        terminal.error_log("Pagefind search index is stale.")
        for failure in failures:
            terminal.log(f"  - {failure}")
        terminal.log(f"{terminal.dim}Run scripts/generate-search-index.py and retry deploy-www.{terminal.reset}")
        return 1

    terminal.ok_log(f"Pagefind search index is current ({fmt_int(len(files))} HTML sources)")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate the Pagefind search index for the static site.")
    parser.add_argument("--check", action="store_true", help="Validate that the generated Pagefind index matches current site HTML.")
    parser.add_argument("--site", default=str(SITE_DIR), help=f"Site directory. Defaults to {SITE_DIR}.")
    parser.add_argument("--output-subdir", default=INDEX_DIR_NAME, help=f"Index output subdirectory. Defaults to {INDEX_DIR_NAME}.")
    parser.add_argument("--pagefind-bin", default=os.environ.get("PAGEFIND_BIN", ""), help="Optional pagefind binary path.")
    parser.add_argument("--json", action="store_true", help="Print machine-readable status and disable terminal styling.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    ensure_cwd()
    terminal = Terminal(json_mode=args.json)
    site_dir = Path(args.site)
    index_dir = site_dir / args.output_subdir

    if args.check:
        return check_current(site_dir, index_dir, terminal)

    terminal.header("Generating Pagefind search index", "Static HTML -> www/pagefind")
    terminal.step_log("Scanning site HTML sources")
    manifest = generate(site_dir, index_dir, args.pagefind_bin, terminal)
    terminal.ok_log(
        f"Indexed {fmt_int(manifest.get('source_file_count'))} HTML files into {index_dir}"
    )
    if args.json:
        print(json.dumps({"ok": True, "output": str(index_dir), "source_file_count": manifest.get("source_file_count")}, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as err:
        Terminal().error_log(str(err))
        raise SystemExit(1)
