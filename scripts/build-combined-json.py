#!/usr/bin/env python3
import datetime
import argparse
import json
import os
import sys


SOURCE_FILES = (
    "aliases.json",
    "db.json",
    "isotopes.json",
    "npm.json",
    "pip.json",
    "stub_exclusions.json",
)
OUTPUT_PATH = os.path.join("data", "combined.json")
SCHEMA_VERSION = 1


def _ensure_cwd():
    scripts_dir = os.path.abspath(os.path.dirname(__file__))
    root = os.path.dirname(scripts_dir)
    os.chdir(root)


def _source_key(path):
    return os.path.splitext(os.path.basename(path))[0]


def _read_json(path):
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def _prune(value):
    if isinstance(value, dict):
        pruned = {}
        for key, child in value.items():
            child = _prune(child)
            if child is not None:
                pruned[key] = child
        return pruned or None
    if isinstance(value, list):
        pruned = []
        for child in value:
            child = _prune(child)
            if child is not None:
                pruned.append(child)
        return pruned or None
    if value is None:
        return None
    return value


def _load_sources():
    sources = {}
    for name in SOURCE_FILES:
        path = os.path.join("data", name)
        if not os.path.exists(path):
            raise FileNotFoundError(path)
        source = _prune(_read_json(path))
        if name == "aliases.json" and source is None:
            source = {}
        sources[_source_key(path)] = source
    _validate_sources(sources)
    return sources


def _expected_combined():
    return {
        "schema": SCHEMA_VERSION,
        "sources": _load_sources(),
    }


def _load_combined(path):
    if not os.path.exists(path):
        raise FileNotFoundError(path)
    combined = _read_json(path)
    if not isinstance(combined, dict):
        raise ValueError(f"{path} must contain an object")
    return combined


def _validate_combined(path):
    combined = _load_combined(path)
    expected = _expected_combined()

    if combined.get("schema") != expected["schema"]:
        raise ValueError(
            f"{path} has schema {combined.get('schema')!r}; expected {SCHEMA_VERSION}"
        )
    if not combined.get("generated_at"):
        raise ValueError(f"{path} is missing generated_at")
    if combined.get("sources") != expected["sources"]:
        raise ValueError(
            f"{path} is stale; regenerate it from local data sources with "
            "scripts/build-combined-json.py"
        )


def _validate_sources(sources):
    db = sources.get("db")
    if not isinstance(db, dict):
        raise ValueError("data/db.json must contain an object")
    casks = db.get("casks")
    if not isinstance(casks, dict) or not casks:
        raise ValueError("data/db.json must contain supported cask metadata")
    for executable, provider in (db.get("entries") or {}).items():
        if not isinstance(provider, str) or not provider.startswith("cask:"):
            continue
        cask = provider[len("cask:") :]
        if cask not in casks:
            raise ValueError(
                f"data/db.json entry {executable!r} points at missing cask {cask!r}"
            )


def _parse_args():
    parser = argparse.ArgumentParser(
        description="Build or validate data/combined.json from local source data."
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Validate that the output already matches local source data.",
    )
    parser.add_argument(
        "--output",
        default=OUTPUT_PATH,
        help=f"Path to write or validate. Defaults to {OUTPUT_PATH}.",
    )
    return parser.parse_args()


def main():
    args = _parse_args()
    _ensure_cwd()
    if args.check:
        try:
            _validate_combined(args.output)
        except (FileNotFoundError, ValueError, json.JSONDecodeError) as err:
            print(f"Invalid {args.output}: {err}", file=sys.stderr)
            return 1
        print(f"OK {args.output} is current")
        return 0

    try:
        combined = {
            "schema": SCHEMA_VERSION,
            "generated_at": datetime.datetime.now(
                datetime.timezone.utc
            ).isoformat(),
            "sources": _expected_combined()["sources"],
        }
    except (FileNotFoundError, ValueError, json.JSONDecodeError) as err:
        print(f"Failed to build {args.output}: {err}", file=sys.stderr)
        return 1

    os.makedirs(os.path.dirname(args.output), exist_ok=True)
    with open(args.output, "w", encoding="utf-8") as handle:
        json.dump(combined, handle, indent=2, sort_keys=True)
        handle.write("\n")

    print(f"Wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
