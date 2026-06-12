from __future__ import annotations

import os
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent
AV_DB_ROOT = Path(os.environ.get("AV_DB_ROOT", PROJECT_ROOT.parent / "av.db")).expanduser()
AV_DB_DATA_DIR = Path(os.environ.get("AV_DB_DATA_DIR", AV_DB_ROOT / "data")).expanduser()
AV_DB_CACHE_DIR = Path(os.environ.get("AV_DB_CACHE_DIR", AV_DB_ROOT / "cache")).expanduser()

COMBINED_DB_PATH = Path(
    os.environ.get(
        "AV_COMBINED_DB_PATH",
        AV_DB_CACHE_DIR / "automic-vault/combined.json",
    )
).expanduser()
DB_JSON_PATH = Path(
    os.environ.get(
        "AV_DB_AUTHORITY_PATH",
        AV_DB_CACHE_DIR / "automic-vault/db.json",
    )
).expanduser()
ISOTOPE_METADATA_PATH = Path(
    os.environ.get(
        "AUTOMIC_VAULT_ISOTOPES_JSON",
        AV_DB_CACHE_DIR / "automic-vault/isotopes.json",
    )
).expanduser()
ISOTOPE_DATA_ROOT = Path(
    os.environ.get("AUTOMIC_VAULT_REPO_CACHE", AV_DB_DATA_DIR / "isotopes")
).expanduser()
RADIOISOTOPE_DATA_ROOT = Path(
    os.environ.get("AUTOMIC_VAULT_RADIOISOTOPES_REPO", AV_DB_DATA_DIR / "radioisotopes")
).expanduser()


def av_db_data_path(*parts: str) -> Path:
    return AV_DB_DATA_DIR.joinpath(*parts)


def av_db_cache_path(*parts: str) -> Path:
    return AV_DB_CACHE_DIR.joinpath(*parts)
