#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = REPO_ROOT / "cache" / "website-inputs.json"
PRODUCT_VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$")
SCAN_LOG_ROW_RE = re.compile(r"^\|\s*[0-9]+\s*\|")


def read_product_version(path: Path) -> str:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as err:
        raise SystemExit(f"Could not read product version source {path}: {err}") from err

    match = re.search(r'^version\s*=\s*"([^"]+)"\s*$', text, re.MULTILINE)
    if not match:
        raise SystemExit(f"Could not find package version in {path}")

    version = match.group(1)
    if not PRODUCT_VERSION_RE.fullmatch(version):
        raise SystemExit(f"Unexpected product version in {path}: {version}")
    return version


def count_scanned_packages(path: Path) -> int:
    try:
        count = sum(1 for line in path.read_text(encoding="utf-8").splitlines() if SCAN_LOG_ROW_RE.match(line))
    except OSError as err:
        raise SystemExit(f"Could not read scan log {path}: {err}") from err

    if count <= 0:
        raise SystemExit(f"Could not find scan log entries in {path}")
    return count


def website_inputs() -> dict[str, Any]:
    return {
        "schemaVersion": 1,
        "generatedAt": dt.datetime.now(dt.UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "productVersion": read_product_version(REPO_ROOT / "Cargo.toml"),
        "scannedPackageCount": count_scanned_packages(REPO_ROOT / "data" / "radioisotopes" / "SCAN_LOG.md"),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Export product-owned inputs for the static website repository.")
    parser.add_argument(
        "--output",
        default=str(DEFAULT_OUTPUT),
        help=f"Output JSON path. Use '-' for stdout. Defaults to {DEFAULT_OUTPUT}.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    payload = website_inputs()
    encoded = json.dumps(payload, indent=2, sort_keys=True) + "\n"

    if args.output == "-":
        sys.stdout.write(encoded)
        return 0

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(encoded, encoding="utf-8")
    print(output_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
