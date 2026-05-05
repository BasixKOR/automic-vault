#!/usr/bin/env python3
import json
import os


OUTPUT_PATH = os.path.join("data", "combined.json")


def _ensure_cwd():
    scripts_dir = os.path.abspath(os.path.dirname(__file__))
    root = os.path.dirname(scripts_dir)
    os.chdir(root)


def _popularity(rank):
    return {
        "installs_per_365_days": 1000 - rank,
        "rank": rank,
    }


def _formula(summary, aliases=None, popularity_rank=None, last_updated_at=None):
    formula = {
        "summary": summary,
        "aliases": aliases or [],
        "oldnames": [],
    }
    if popularity_rank is not None:
        formula["popularity"] = _popularity(popularity_rank)
    if last_updated_at is not None:
        formula["last_updated_at"] = last_updated_at
    return formula


def _cask(summary, popularity_rank, last_updated_at):
    return {
        "summary": summary,
        "homepage": "https://example.test/automic-fixture",
        "aliases": [],
        "url": "https://example.test/automic-fixture.zip",
        "sha256": "0" * 64,
        "version": "1.0.0",
        "dependencies": [],
        "binaries": [],
        "popularity": _popularity(popularity_rank),
        "last_updated_at": last_updated_at,
    }


def main():
    _ensure_cwd()
    data = {
        "schema": 1,
        "generated_at": "2026-05-05T00:00:00Z",
        "sources": {
            "aliases": {
                "clawhub": "npm:clawhub",
                "openclaw": "npm:openclaw",
                "qmd": "npm:@tobilu/qmd",
            },
            "db": {
                "schema": 6,
                "generated_at": "2026-05-05T00:00:00Z",
                "entries": {
                    "aws": "awscli",
                    "aws_completer": "awscli",
                    "bash": "bash",
                    "gh": "gh",
                    "libpq": "libpq",
                    "rg": "ripgrep",
                    "sqlite": "sqlite",
                },
                "formulas": {
                    "awscli": _formula("Official Amazon AWS command-line interface"),
                    "bash": _formula("Bourne-Again SHell"),
                    "gh": _formula("GitHub command-line tool"),
                    "libpq": _formula("Postgres client library"),
                    "node": _formula(
                        "JavaScript runtime",
                        aliases=["node@25"],
                        popularity_rank=3,
                        last_updated_at="2026-05-03T00:00:00Z",
                    ),
                    "ripgrep": _formula(
                        "Search tool",
                        aliases=["rg"],
                        popularity_rank=1,
                        last_updated_at="2026-05-05T00:00:00Z",
                    ),
                    "sqlite": _formula(
                        "SQL database engine",
                        popularity_rank=2,
                        last_updated_at="2026-05-04T00:00:00Z",
                    ),
                },
                "casks": {
                    "automic-fixture": _cask(
                        "Fixture cask",
                        popularity_rank=4,
                        last_updated_at="2026-05-02T00:00:00Z",
                    ),
                },
            },
            "isotopes": {
                "aws-cli": {
                    "name": "isotope:aws-cli",
                    "modifies": "brew:awscli",
                    "migrate": "aws configure import --csv file://$1",
                    "version": "1.0.0",
                },
                "gh": {
                    "name": "isotope:gh",
                    "replaces": "brew:gh",
                    "migrate": "/opt/isotopes/gh/bin/gh auth av-migrate \"$@\"",
                    "version": "2.80.0",
                },
            },
            "npm": {
                "openclaw": {"homebrewDeps": ["sqlite"]},
                "qmd": {"homebrewDeps": ["sqlite"]},
            },
            "pip": {
                "psycopg2": {
                    "homebrewDeps": ["libpq"],
                    "pythonFormula": "python@3.12",
                },
            },
            "stub_exclusions": {
                "brew:bash": ["bashbug"],
                "brew:ffmpeg": ["ffmpeg-unused"],
                "brew:ffmpeg-full": ["ffmpeg-unused"],
            },
        },
    }
    os.makedirs(os.path.dirname(OUTPUT_PATH), exist_ok=True)
    with open(OUTPUT_PATH, "w", encoding="utf-8") as handle:
        json.dump(data, handle, indent=2, sort_keys=True)
        handle.write("\n")
    print(f"Wrote {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
