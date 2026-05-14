#!/usr/bin/env python3
import json
import os


OUTPUT_PATHS = [
    os.path.join("src", "lib", "rs", "fixtures", "coverage-combined.json"),
    os.path.join("data", "combined.json"),
]


def _ensure_cwd():
    scripts_dir = os.path.abspath(os.path.dirname(__file__))
    root = os.path.dirname(scripts_dir)
    os.chdir(root)


def _popularity(rank):
    return {
        "installs_per_365_days": 1000 - rank,
        "rank": rank,
    }


def _npm_popularity(rank):
    return {
        "downloads_per_30_days": 1000 - rank,
        "rank": rank,
    }


def _formula(
    summary,
    aliases=None,
    popularity_rank=None,
    popularity_installs=None,
    last_updated_at=None,
    pulse_kind=None,
):
    formula = {
        "summary": summary,
        "aliases": aliases or [],
        "oldnames": [],
    }
    if popularity_rank is not None and popularity_installs is not None:
        formula["popularity"] = {
            "installs_per_365_days": popularity_installs,
            "rank": popularity_rank,
        }
    elif popularity_rank is not None:
        formula["popularity"] = _popularity(popularity_rank)
    if last_updated_at is not None:
        formula["last_updated_at"] = last_updated_at
    if pulse_kind is not None:
        formula["pulse_kind"] = pulse_kind
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


def _npm(summary, version, executable, popularity_rank, last_updated_at):
    return {
        "summary": summary,
        "homepage": "https://example.test/npm",
        "version": version,
        "executable": executable,
        "popularity": _npm_popularity(popularity_rank),
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
                "schema": 7,
                "generated_at": "2026-05-05T00:00:00Z",
                "entries": {
                    "aws": "awscli",
                    "aws_completer": "awscli",
                    "bash": "bash",
                    "coverage-npm": "npm:coverage-npm",
                    "gh": "gh",
                    "libpq": "libpq",
                    "scoped-tool": "npm:@scope/scoped-tool",
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
                    "portable-libffi": _formula(
                        "Portable Foreign Function Interface library",
                        popularity_rank=26875,
                        popularity_installs=2,
                        last_updated_at="2025-09-30T16:01:57+01:00",
                        pulse_kind="new",
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
                    "sqlite-utils": _formula(
                        "SQLite utility collection",
                        popularity_rank=5,
                        last_updated_at="2026-05-01T00:00:00Z",
                        pulse_kind="new",
                    ),
                },
                "casks": {
                    "automic-fixture": _cask(
                        "Fixture cask",
                        popularity_rank=4,
                        last_updated_at="2026-05-02T00:00:00Z",
                    ),
                    "codex": _cask(
                        "OpenAI Codex",
                        popularity_rank=6,
                        last_updated_at="2026-05-01T00:00:00Z",
                    ),
                },
                "npms": {
                    "coverage-npm": _npm(
                        "Coverage npm tool",
                        version="1.2.3",
                        executable="coverage-npm",
                        popularity_rank=7,
                        last_updated_at="2026-05-01T00:00:00Z",
                    ),
                    "@scope/scoped-tool": _npm(
                        "Scoped npm tool",
                        version="2.0.0",
                        executable="scoped-tool",
                        popularity_rank=8,
                        last_updated_at="2026-05-01T00:00:00Z",
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
                "coverage-npm": {"homebrewDeps": ["node"]},
                "openclaw": {"homebrewDeps": ["sqlite"]},
                "qmd": {"homebrewDeps": ["sqlite"]},
            },
            "pip": {
                "coverage-pip": {
                    "homebrewDeps": [],
                    "pythonFormula": "python@3.14",
                },
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
    _validate(data)
    for output_path in OUTPUT_PATHS:
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        with open(output_path, "w", encoding="utf-8") as handle:
            json.dump(data, handle, indent=2, sort_keys=True)
            handle.write("\n")
        print(f"Wrote {output_path}")


def _validate(data):
    sources = data["sources"]
    db = sources["db"]
    assert db["schema"] == 7
    assert db["formulas"]["ripgrep"]["aliases"] == ["rg"]
    assert db["formulas"]["node"]["aliases"] == ["node@25"]
    assert db["casks"]["codex"]["version"]
    assert db["entries"]["coverage-npm"] == "npm:coverage-npm"
    assert db["entries"]["scoped-tool"] == "npm:@scope/scoped-tool"
    assert db["npms"]["coverage-npm"]["executable"] == "coverage-npm"
    assert sources["isotopes"]["gh"]["replaces"] == "brew:gh"
    assert sources["isotopes"]["aws-cli"]["modifies"] == "brew:awscli"
    assert sources["npm"]["coverage-npm"]["homebrewDeps"] == ["node"]
    assert sources["pip"]["coverage-pip"]["pythonFormula"] == "python@3.14"


if __name__ == "__main__":
    main()
