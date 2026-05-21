#!/usr/bin/env python3
"""Prepare a local Automic Vault isotope fork clone for a GitHub repo."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


def run(args: list[str], cwd: Path | None = None, dry_run: bool = False) -> str:
    printable = " ".join(args)
    if cwd is not None:
        printable = f"(cd {cwd} && {printable})"
    if dry_run:
        print(printable)
        return ""
    completed = subprocess.run(
        args,
        cwd=cwd,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if completed.stderr:
        sys.stderr.write(completed.stderr)
    return completed.stdout.strip()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def parse_repo(value: str) -> tuple[str, str]:
    value = value.strip()
    patterns = [
        r"^https://github\.com/([^/]+)/([^/.]+)(?:\.git)?/?$",
        r"^git@github\.com:([^/]+)/([^/.]+)(?:\.git)?$",
        r"^([^/\s]+)/([^/\s]+)$",
    ]
    for pattern in patterns:
        match = re.match(pattern, value)
        if match:
            return match.group(1), match.group(2)
    raise SystemExit(f"invalid GitHub repo: {value!r}; expected owner/name or GitHub URL")


def default_fork_name(owner: str, repo: str) -> str:
    if repo in {"cli", "cmd", "tool", "app"}:
        return f"{owner}-{repo}"
    return repo


def ensure_clean_if_exists(path: Path, dry_run: bool) -> None:
    if dry_run:
        return
    if not (path / ".git").is_dir():
        if path.exists():
            raise SystemExit(f"clone path exists but is not a git repo: {path}")
        return
    status = run(["git", "status", "--porcelain"], cwd=path)
    if status:
        raise SystemExit(f"refusing to alter dirty isotope clone: {path}")


def repo_exists(slug: str, dry_run: bool) -> bool:
    if dry_run:
        return False
    result = subprocess.run(
        ["gh", "repo", "view", slug, "--json", "nameWithOwner"],
        text=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return result.returncode == 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("repo", help="Upstream GitHub repo, e.g. supabase/cli")
    parser.add_argument("--org", default="automic-vault")
    parser.add_argument("--fork-name", help="Fork repo name under --org")
    parser.add_argument("--clone-root", type=Path, default=repo_root() / "data" / "isotopes")
    parser.add_argument("--branch", default="trunk")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    owner, repo = parse_repo(args.repo)
    fork_name = args.fork_name or default_fork_name(owner, repo)
    upstream_slug = f"{owner}/{repo}"
    fork_slug = f"{args.org}/{fork_name}"
    clone_dir = args.clone_root / fork_name

    if args.dry_run:
        print(f"upstream: {upstream_slug}")
        print(f"fork:     {fork_slug}")
        print(f"clone:    {clone_dir}")

    if not args.dry_run:
        run(["gh", "auth", "status"])
    if not repo_exists(fork_slug, args.dry_run):
        run(
            [
                "gh",
                "repo",
                "fork",
                upstream_slug,
                "--org",
                args.org,
                "--fork-name",
                fork_name,
                "--clone=false",
                "--default-branch-only",
            ],
            dry_run=args.dry_run,
        )

    if not args.dry_run:
        args.clone_root.mkdir(parents=True, exist_ok=True)
    ensure_clean_if_exists(clone_dir, args.dry_run)
    if not clone_dir.exists():
        run(
            [
                "git",
                "clone",
                "--depth",
                "1",
                f"https://github.com/{upstream_slug}.git",
                str(clone_dir),
            ],
            dry_run=args.dry_run,
        )

    origin_url = f"git@github.com:{fork_slug}.git"
    upstream_url = f"https://github.com/{upstream_slug}.git"
    remotes = run(["git", "remote"], cwd=clone_dir, dry_run=args.dry_run).splitlines()
    if "upstream" not in remotes:
        run(["git", "remote", "add", "upstream", upstream_url], cwd=clone_dir, dry_run=args.dry_run)
    else:
        run(["git", "remote", "set-url", "upstream", upstream_url], cwd=clone_dir, dry_run=args.dry_run)
    if "origin" not in remotes:
        run(["git", "remote", "add", "origin", origin_url], cwd=clone_dir, dry_run=args.dry_run)
    else:
        run(["git", "remote", "set-url", "origin", origin_url], cwd=clone_dir, dry_run=args.dry_run)
    run(["git", "checkout", "-B", args.branch], cwd=clone_dir, dry_run=args.dry_run)

    print(clone_dir)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
