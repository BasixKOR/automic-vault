#!/usr/bin/env python3
import datetime
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
        sources[_source_key(path)] = _prune(_read_json(path))
    _validate_sources(sources)
    return sources


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


def main():
    _ensure_cwd()
    try:
        combined = {
            "schema": SCHEMA_VERSION,
            "generated_at": datetime.datetime.now(
                datetime.timezone.utc
            ).isoformat(),
            "sources": _load_sources(),
        }
    except (FileNotFoundError, ValueError, json.JSONDecodeError) as err:
        print(f"Failed to build {OUTPUT_PATH}: {err}", file=sys.stderr)
        return 1

    os.makedirs(os.path.dirname(OUTPUT_PATH), exist_ok=True)
    with open(OUTPUT_PATH, "w", encoding="utf-8") as handle:
        json.dump(combined, handle, indent=2, sort_keys=True)
        handle.write("\n")

    print(f"Wrote {OUTPUT_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
