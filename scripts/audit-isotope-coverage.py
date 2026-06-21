#!/usr/bin/env python3
import argparse
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DB = ROOT.parent / "av.db/cache/automic-vault/combined.json"
REMOTE_DB = Path("/var/db/automic-vault/db.json")
DEFAULT_ISOTOPE_ROOTS = [ROOT.parent / "isotopes", ROOT.parent / "radioisotopes"]

SECRET_TERMS = {
    "api key": 5,
    "apikey": 5,
    "auth": 4,
    "credential": 5,
    "keychain": 4,
    "login": 3,
    "oauth": 5,
    "password": 5,
    "secret": 5,
    "ssh": 4,
    "token": 5,
}


def norm(value):
    if not value:
        return None
    value = str(value).strip().lower()
    if not value:
        return None
    for prefix in ("brew:", "cask:", "npm:", "pip:", "pypi:", "isotope:"):
        if value.startswith(prefix):
            value = value[len(prefix):]
    return value or None


def cli_stems(value):
    value = norm(value)
    if not value:
        return set()
    stems = {value}
    if value.endswith("-cli") and len(value) > 4:
        stems.add(value[:-4])
    elif value.endswith("cli") and len(value) > 3:
        stems.add(value[:-3])
    return stems


def leaf(value):
    value = norm(value)
    if not value:
        return None
    return value.rstrip("/").rsplit("/", 1)[-1]


def identifiers(name, metadata):
    ids = set(cli_stems(name))
    for key in ("aliases", "oldnames", "binaries"):
        for value in metadata.get(key, []) or []:
            ids.update(cli_stems(value))
    for key in (
        "executable",
        "repository",
        "upstreamRepository",
        "homepage",
        "replaces",
        "modifies",
        "name",
    ):
        value = metadata.get(key)
        ids.update(cli_stems(value))
        ids.update(cli_stems(leaf(value)))
    return {value for value in ids if value}


def isotope_identifiers(combined, isotope_roots):
    ids = set()
    for name, metadata in combined.get("sources", {}).get("isotopes", {}).items():
        ids.update(identifiers(name, metadata))

    for root in isotope_roots:
        if root.is_dir():
            ids.update(
                stem
                for child in root.iterdir()
                if child.is_dir() and not child.name.startswith(".")
                for stem in cli_stems(child.name)
            )
    return ids


def package_rows(combined):
    seen = set()
    sources_data = combined.get("sources", {})
    db = sources_data.get("db", {})
    sources = [
        ("brew", db.get("formulas", {})),
        ("cask", db.get("casks", {})),
        ("npm", db.get("npms", {})),
        ("pypi", db.get("pypi", db.get("pips", {}))),
        ("npm", sources_data.get("npm", {})),
        ("pypi", sources_data.get("pip", {})),
    ]
    for source, packages in sources:
        for name, metadata in packages.items():
            key = (source, name)
            if key in seen:
                continue
            seen.add(key)
            metadata = metadata or {}
            yield {
                "source": source,
                "name": name,
                "summary": metadata.get("summary") or metadata.get("description") or "",
                "ids": identifiers(name, metadata),
            }


def score(row):
    text = " ".join([row["name"], row["summary"], *sorted(row["ids"])]).lower()
    return sum(weight for term, weight in SECRET_TERMS.items() if term in text)


def audit(combined, isotope_roots):
    covered_ids = isotope_identifiers(combined, isotope_roots)
    rows = list(package_rows(combined))
    covered = [row for row in rows if row["ids"] & covered_ids]
    candidates = []
    for row in rows:
        if row in covered:
            continue
        row_score = score(row)
        if row_score:
            candidates.append((row_score, row))
    candidates.sort(key=lambda item: (-item[0], item[1]["source"], item[1]["name"]))
    install_names = len(combined.get("sources", {}).get("db", {}).get("entries", {}))
    return rows, install_names, covered, covered_ids, candidates


def print_audit(rows, install_names, covered, covered_ids, candidates, limit):
    print(f"package records: {len(rows)}")
    print(f"install names: {install_names}")
    print(f"isotope identifiers: {len(covered_ids)}")
    print(f"packages already matched by isotope identifiers: {len(covered)}")
    print(f"uncovered credential-looking packages: {len(candidates)}")
    print()
    for row_score, row in candidates[:limit]:
        summary = " ".join(row["summary"].split())
        print(f"{row_score:>2}  {row['source']:<4}  {row['name']:<32}  {summary[:96]}")


def self_check():
    fixture = ROOT / "src/lib/rs/fixtures/coverage-combined.json"
    combined = json.loads(fixture.read_text())
    rows, install_names, covered, covered_ids, candidates = audit(combined, [])
    assert rows
    assert install_names
    assert "gh" in covered_ids
    assert any(row["name"] == "gh" for row in covered)
    assert all(row["name"] != "gh" for _, row in candidates)


def main():
    parser = argparse.ArgumentParser(description="Audit package DB isotope coverage.")
    parser.add_argument("combined_json", nargs="?", type=Path, default=REMOTE_DB)
    parser.add_argument("--limit", type=int, default=80)
    parser.add_argument("--self-check", action="store_true")
    args = parser.parse_args()

    if args.self_check:
        self_check()
        return

    if not args.combined_json.is_file():
        fallback = DEFAULT_DB if DEFAULT_DB.is_file() else None
        if fallback is None:
            sys.exit(f"missing combined package DB: {args.combined_json}")
        args.combined_json = fallback

    combined = json.loads(args.combined_json.read_text())
    print_audit(*audit(combined, DEFAULT_ISOTOPE_ROOTS), args.limit)


if __name__ == "__main__":
    main()
