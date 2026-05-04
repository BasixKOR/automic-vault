#!/usr/bin/env python3
import base64
import datetime
import hashlib
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed

FORMULA_URL = "https://formulae.brew.sh/api/formula.json"
ANALYTICS_URL = "https://formulae.brew.sh/api/analytics/install/365d.json"
CASKS_URL = "https://formulae.brew.sh/api/cask.json"
CASK_ANALYTICS_URL = "https://formulae.brew.sh/api/analytics/cask-install/365d.json"
CACHE_DIR = "cache"
ECOSYSTEM = "brew.sh"
DB_PATH = os.path.join("data", "db.json")
SCHEMA_VERSION = 6
HOMEWBREW_CORE_REPO = "Homebrew/homebrew-core"
HOMEWBREW_CASK_REPO = "Homebrew/homebrew-cask"
META_KEY = "__pkgdb_meta__"
PAYLOAD_KEY = "__pkgdb_payload__"
USER_AGENT = "nucleus/0.1"
CHECK_INTERVAL_SECONDS = 24 * 60 * 60
DEFAULT_TIMEOUT = 60
MANIFEST_ACCEPT = "application/vnd.oci.image.index.v1+json"
TOKEN_SERVICE = "https://ghcr.io/token"
FORCE_REFRESH = False

_GHCR_TOKENS = {}


def _ensure_cwd():
    scripts_dir = os.path.abspath(os.path.dirname(__file__))
    root = os.path.dirname(scripts_dir)
    os.chdir(root)


def _cache_path(url):
    digest = hashlib.sha256(url.encode("utf-8")).hexdigest()
    return os.path.join(CACHE_DIR, ECOSYSTEM, f"{digest}.json")


def _read_cached_json(url):
    path = _cache_path(url)
    if not os.path.exists(path):
        raise FileNotFoundError(path)
    with open(path, "rb") as handle:
        data = json.load(handle)
    if isinstance(data, dict) and META_KEY in data and PAYLOAD_KEY in data:
        meta = data.get(META_KEY) or {}
        return data.get(PAYLOAD_KEY), meta
    return data, {}


def _write_cache(path, payload, etag, checked_at):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    wrapper = {
        META_KEY: {"etag": etag, "checked_at": checked_at},
        PAYLOAD_KEY: payload,
    }
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(wrapper, handle)


def _fetch_json(url, github_token=None):
    path = _cache_path(url)
    payload = None
    meta = {}
    if os.path.exists(path):
        payload, meta = _read_cached_json(url)

    checked_at = meta.get("checked_at")
    now = int(time.time())
    if (
        not FORCE_REFRESH
        and isinstance(checked_at, int)
        and now - checked_at < CHECK_INTERVAL_SECONDS
    ):
        return payload

    headers = {"Accept": "application/json", "User-Agent": USER_AGENT}
    parsed = urllib.parse.urlparse(url)
    if parsed.hostname == "ghcr.io":
        headers["Accept"] = MANIFEST_ACCEPT
        repo = _ghcr_repo_from_url(parsed.path)
        if repo:
            token = _ghcr_bearer_token(repo, github_token)
            if token:
                headers["Authorization"] = f"Bearer {token}"
    elif parsed.hostname == "api.github.com":
        headers["Accept"] = "application/vnd.github+json"
        if github_token:
            headers["Authorization"] = f"Bearer {github_token}"
    etag = meta.get("etag")
    if etag:
        headers["If-None-Match"] = etag

    request = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=DEFAULT_TIMEOUT) as response:
            data = response.read()
            etag = response.headers.get("etag")
            payload = json.loads(data)
            _write_cache(path, payload, etag, now)
            return payload
    except urllib.error.HTTPError as err:
        if err.code == 404:
            return None
        if err.code == 304 and payload is not None:
            _write_cache(path, payload, etag, now)
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


def _github_token():
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        return token.strip()
    try:
        result = subprocess.run(
            ["gh", "auth", "token"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (FileNotFoundError, subprocess.CalledProcessError):
        return None
    token = result.stdout.strip()
    if token:
        return token
    return None


def _github_username():
    for key in ("GHCR_USERNAME", "GITHUB_ACTOR", "USER"):
        value = os.environ.get(key)
        if value:
            return value.strip()
    try:
        result = subprocess.run(
            ["gh", "api", "user", "-q", ".login"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (FileNotFoundError, subprocess.CalledProcessError):
        return None
    username = result.stdout.strip()
    if username:
        return username
    return None


def _ghcr_repo_from_url(path):
    parts = [part for part in path.split("/") if part]
    if len(parts) < 4 or parts[0] != "v2":
        return None
    return "/".join(parts[1:-2])


def _ghcr_bearer_token(repo, github_token):
    now = int(time.time())
    cached = _GHCR_TOKENS.get(repo)
    if cached and cached["expires_at"] > now:
        return cached["token"]

    if not github_token:
        return None
    username = _github_username() or "x-access-token"
    scope = f"repository:{repo}:pull"
    query = urllib.parse.urlencode({"service": "ghcr.io", "scope": scope})
    url = f"{TOKEN_SERVICE}?{query}"
    basic = base64.b64encode(
        f"{username}:{github_token}".encode("utf-8")
    ).decode("utf-8")
    headers = {
        "Authorization": f"Basic {basic}",
        "User-Agent": USER_AGENT,
    }
    request = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=DEFAULT_TIMEOUT) as response:
            data = json.loads(response.read())
    except urllib.error.HTTPError as err:
        print(f"Failed to get GHCR token for {repo}: {err}", file=sys.stderr)
        return None
    bearer = data.get("token")
    expires_in = data.get("expires_in", 300)
    if bearer:
        _GHCR_TOKENS[repo] = {
            "token": bearer,
            "expires_at": now + int(expires_in) - 10,
        }
        return bearer
    return None


def _stable_version(stable):
    if isinstance(stable, str):
        return stable
    if isinstance(stable, dict):
        for key in ("version", "tag"):
            value = stable.get(key)
            if value:
                return value
    return None


def _manifest_url(formula):
    name = formula.get("name")
    versions = formula.get("versions", {})
    stable = versions.get("stable")
    version = _stable_version(stable)
    if not name or not version:
        return None

    url = (
        "https://ghcr.io/v2/homebrew/core/"
        f"{name.replace('+', 'x')}/manifests/{version}"
    )

    revision = formula.get("revision")
    stable_revision = None
    if isinstance(stable, dict):
        stable_revision = stable.get("revision")
    revision_value = revision if revision is not None else stable_revision
    if revision_value not in (None, 0):
        url = f"{url}_{revision_value}"

    rebuild = formula.get("bottle", {}).get("stable", {}).get("rebuild")
    if rebuild:
        url = f"{url}-{rebuild}"

    return url


def _parse_count(value):
    if value is None:
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, str):
        value = value.replace(",", "")
        if value.isdigit():
            return int(value)
    return None


def _fetch_popularity(github_token):
    payload = _fetch_json(ANALYTICS_URL, github_token)
    items = payload.get("items") if isinstance(payload, dict) else None
    popularity = {}
    if not isinstance(items, list):
        return popularity
    for item in items:
        if not isinstance(item, dict):
            continue
        formula = item.get("formula")
        count = _parse_count(item.get("count"))
        rank = _parse_count(item.get("number"))
        if formula and count is not None and rank is not None:
            popularity[formula] = {
                "installs_per_365_days": count,
                "rank": rank,
            }
    return popularity


def _fetch_cask_popularity(github_token):
    payload = _fetch_json(CASK_ANALYTICS_URL, github_token)
    items = payload.get("items") if isinstance(payload, dict) else None
    popularity = {}
    if not isinstance(items, list):
        return popularity
    for item in items:
        if not isinstance(item, dict):
            continue
        cask = item.get("cask")
        count = _parse_count(item.get("count"))
        rank = _parse_count(item.get("number"))
        if cask and count is not None and rank is not None:
            popularity[cask] = {
                "installs_per_365_days": count,
                "rank": rank,
            }
    return popularity


def _parse_exec_paths(paths):
    executables = set()
    for entry in paths:
        if not entry:
            continue
        entry = entry.strip()
        if not entry:
            continue
        name = entry.rsplit("/", 1)[-1]
        if name:
            executables.add(name)
    return executables


def _formula_metadata(formula):
    name = formula.get("name")
    if not name:
        return None
    return {
        "summary": formula.get("desc") or "",
        "aliases": formula.get("aliases") or [],
        "oldnames": formula.get("oldnames") or [],
    }


def _cask_url(token):
    return f"https://formulae.brew.sh/api/cask/{token}.json"


def _formula_source_path(name):
    return f"Formula/{name[0]}/{name}.rb"


def _cask_source_path(token):
    return f"Casks/{token[0]}/{token}.rb"


def _git_repo_cache_path(repo):
    return os.path.join(CACHE_DIR, ECOSYSTEM, "git", repo.rsplit("/", 1)[-1])


def _ensure_git_repo(repo):
    path = _git_repo_cache_path(repo)
    url = f"https://github.com/{repo}.git"
    if not os.path.exists(path):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        subprocess.run(
            ["git", "clone", "--filter=blob:none", "--no-checkout", url, path],
            check=True,
            capture_output=True,
            text=True,
        )
    else:
        subprocess.run(
            ["git", "-C", path, "fetch", "--quiet", "--filter=blob:none", "origin"],
            check=True,
            capture_output=True,
            text=True,
        )
    return path


def _git_default_revision(repo_path):
    candidates = [
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/main",
        "refs/remotes/origin/master",
    ]
    for candidate in candidates:
        result = subprocess.run(
            ["git", "-C", repo_path, "rev-parse", "--verify", candidate],
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            return candidate
    raise RuntimeError(f"Unable to resolve a fetched revision for {repo_path}")


def _git_last_updated_at(repo, keyed_paths, scope):
    if not keyed_paths:
        return {}

    repo_path = _ensure_git_repo(repo)
    revision = _git_default_revision(repo_path)
    pending = set(keyed_paths.keys())
    updates = {}
    current_date = None
    command = [
        "git",
        "-C",
        repo_path,
        "log",
        revision,
        "--format=__DATE__%cI",
        "--name-only",
        "--",
        scope,
    ]
    process = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        for raw_line in process.stdout:
            line = raw_line.rstrip("\n")
            if not line:
                continue
            if line.startswith("__DATE__"):
                current_date = line[len("__DATE__") :]
                continue
            if current_date is None or line not in pending:
                continue
            updates[keyed_paths[line]] = current_date
            pending.remove(line)
            if len(updates) % 100 == 0:
                print(
                    f"Resolved {len(updates)}/{len(keyed_paths)} git update timestamps for {repo}",
                    file=sys.stderr,
                )
            if not pending:
                process.terminate()
                break
    finally:
        stdout, stderr = process.communicate()
        if process.returncode not in (0, -15):
            message = stderr.strip() or stdout.strip() or f"git log failed for {repo}"
            raise RuntimeError(message)
    return updates


def _parse_binary_artifact(artifact):
    if not isinstance(artifact, dict) or len(artifact) != 1:
        return None
    if "binary" not in artifact:
        return None

    value = artifact["binary"]
    target = None
    if isinstance(value, str):
        source = value
    elif isinstance(value, list) and value:
        source = value[0]
        if len(value) > 1 and isinstance(value[1], dict):
            target = value[1].get("target")
    else:
        return None

    if not isinstance(source, str) or not source:
        return None
    if target is not None and (not isinstance(target, str) or not target):
        return None
    return {"source": source, "target": target}


def _supported_cask_artifacts(artifacts):
    binaries = []
    for artifact in artifacts:
        parsed = _parse_binary_artifact(artifact)
        if parsed is not None:
            binaries.append(parsed)
            continue
        if isinstance(artifact, dict) and set(artifact.keys()) <= {"zap", "uninstall"}:
            continue
        return None
    return binaries if binaries else None


def _cask_metadata(cask):
    token = cask.get("token")
    if not token or cask.get("disabled") or cask.get("deprecated"):
        return None

    binaries = _supported_cask_artifacts(cask.get("artifacts") or [])
    if binaries is None:
        return None

    url = cask.get("url")
    sha256 = cask.get("sha256")
    version = cask.get("version")
    if not isinstance(url, str) or not url:
        return None
    if not isinstance(sha256, str) or not sha256:
        return None
    if not isinstance(version, str) or not version:
        return None

    depends_on = cask.get("depends_on") or {}
    formula_dependencies = depends_on.get("formula") or []
    if not all(isinstance(dep, str) and dep for dep in formula_dependencies):
        return None

    return {
        "summary": cask.get("desc") or "",
        "homepage": cask.get("homepage") or "",
        "aliases": cask.get("old_tokens") or [],
        "url": url,
        "sha256": sha256,
        "version": version,
        "dependencies": sorted(set(formula_dependencies)),
        "binaries": binaries,
    }


def _collect_formula_entries(formulae, popularity_by_formula, manifests, updated_at_by_formula):
    entries = {}
    formulas = {}
    missing_manifests = 0
    for formula in formulae:
        if not isinstance(formula, dict):
            continue
        name = formula.get("name")
        if not name:
            continue

        metadata = _formula_metadata(formula)
        if metadata is not None:
            formulas[name] = metadata

        popularity = popularity_by_formula.get(name)
        updated_at = updated_at_by_formula.get(name)
        if name in formulas:
            if popularity is not None:
                formulas[name]["popularity"] = popularity
            if updated_at is not None:
                formulas[name]["last_updated_at"] = updated_at

        if "@" in name:
            continue

        url = _manifest_url(formula)
        if not url:
            continue

        payload = manifests.get(url)
        if not payload:
            missing_manifests += 1
            continue

        manifest_list = payload.get("manifests", [])
        executables = set()
        for manifest in manifest_list:
            annotations = None
            if isinstance(manifest, dict):
                annotations = manifest.get("annotations")
            if not annotations:
                continue
            provides = annotations.get("sh.brew.path_exec_files")
            if not provides:
                continue
            paths = [item.strip() for item in provides.split(",") if item.strip()]
            executables.update(_parse_exec_paths(paths))
            if executables:
                break

        for executable in executables:
            entries.setdefault(executable, []).append(
                {
                    "provider": name,
                    "popularity": popularity["installs_per_365_days"]
                    if popularity is not None
                    else 0,
                }
            )

    return entries, formulas, missing_manifests


def _collect_cask_entries(casks, popularity_by_cask, updated_at_by_cask):
    entries = {}
    metadata = {}
    for cask in casks:
        if not isinstance(cask, dict):
            continue
        token = cask.get("token")
        if not token:
            continue

        supported = _cask_metadata(cask)
        if supported is None:
            continue

        popularity = popularity_by_cask.get(token)
        if supported["binaries"] and popularity is not None:
            supported["popularity"] = popularity
        updated_at = updated_at_by_cask.get(token)
        if supported["binaries"] and updated_at is not None:
            supported["last_updated_at"] = updated_at
        metadata[token] = supported
        for binary in supported["binaries"]:
            executable = binary.get("target") or os.path.basename(binary["source"])
            if executable:
                entries.setdefault(executable, []).append(
                    {
                        "provider": f"cask:{token}",
                        "popularity": popularity["installs_per_365_days"]
                        if popularity is not None
                        else 0,
                    }
                )

    return entries, metadata


def _sorted_entries(entries):
    ordered = {}
    for executable in sorted(entries.keys()):
        items = entries[executable]
        items.sort(
            key=lambda item: (
                -(item.get("popularity") or 0),
                item.get("provider", ""),
            )
        )
        top = items[0]["provider"] if items else None
        if top:
            ordered[executable] = top
    return ordered


def _merge_entries(*groups):
    merged = {}
    for group in groups:
        for executable, items in group.items():
            merged.setdefault(executable, []).extend(items)
    return merged


def main():
    global FORCE_REFRESH

    _ensure_cwd()

    for arg in sys.argv[1:]:
        if arg == "--refresh":
            FORCE_REFRESH = True
        elif arg in ("--help", "-h"):
            print("Usage: scripts/build-db.py [--refresh]")
            return
        else:
            print(f"Unknown argument: {arg}", file=sys.stderr)
            print("Usage: scripts/build-db.py [--refresh]", file=sys.stderr)
            sys.exit(2)

    os.makedirs(os.path.join(CACHE_DIR, ECOSYSTEM), exist_ok=True)

    github_token = _github_token()

    formulae = _fetch_json(FORMULA_URL, github_token)
    if not isinstance(formulae, list):
        print("Formula list was not a list.", file=sys.stderr)
        sys.exit(2)
    cask_index = _fetch_json(CASKS_URL, github_token)
    if not isinstance(cask_index, list):
        print("Cask list was not a list.", file=sys.stderr)
        sys.exit(2)

    popularity_by_formula = {}
    popularity_by_cask = {}
    try:
        popularity_by_formula = _fetch_popularity(github_token)
    except Exception as err:
        print(f"Failed to fetch analytics data: {err}", file=sys.stderr)
    try:
        popularity_by_cask = _fetch_cask_popularity(github_token)
    except Exception as err:
        print(f"Failed to fetch cask analytics data: {err}", file=sys.stderr)

    manifest_urls = []
    for formula in formulae:
        if not isinstance(formula, dict):
            continue
        name = formula.get("name")
        if not name or "@" in name:
            continue
        url = _manifest_url(formula)
        if url:
            manifest_urls.append(url)

    cask_urls = []
    for entry in cask_index:
        if not isinstance(entry, dict):
            continue
        token = entry.get("token")
        if token and _cask_metadata(entry) is not None:
            cask_urls.append(_cask_url(token))

    formula_paths = {}
    for formula in formulae:
        if not isinstance(formula, dict):
            continue
        name = formula.get("name")
        if not name:
            continue
        formula_paths[_formula_source_path(name)] = name

    cask_paths = {}
    for entry in cask_index:
        if not isinstance(entry, dict):
            continue
        token = entry.get("token")
        if token and _cask_metadata(entry) is not None:
            cask_paths[_cask_source_path(token)] = token

    manifests = {}
    casks = {}
    completed = 0
    max_workers = min(32, (os.cpu_count() or 4) * 4)
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_map = {}
        for url in manifest_urls:
            future_map[executor.submit(_fetch_json, url, github_token)] = (
                "manifest",
                url,
            )
        for url in cask_urls:
            future_map[executor.submit(_fetch_json, url, github_token)] = (
                "cask",
                url,
            )
        for future in as_completed(future_map):
            kind, key = future_map[future]
            try:
                payload = future.result()
            except Exception as err:
                print(f"Failed to fetch {key}: {err}", file=sys.stderr)
                continue
            if payload:
                if kind == "manifest":
                    manifests[key] = payload
                elif kind == "cask":
                    casks[key] = payload
                elif kind == "formula_commit":
                    updated_at = _commit_updated_at(payload)
                    if updated_at is not None:
                        formula_updates[key] = updated_at
                elif kind == "cask_commit":
                    updated_at = _commit_updated_at(payload)
                    if updated_at is not None:
                        cask_updates[key] = updated_at
            completed += 1
            if completed % 20 == 0:
                print(
                    "Fetched "
                    f"{completed}/"
                    f"{len(manifest_urls) + len(cask_urls)} "
                    "api payloads...",
                    file=sys.stderr,
                )

    formula_updates = _git_last_updated_at(
        HOMEWBREW_CORE_REPO,
        formula_paths,
        "Formula",
    )
    cask_updates = _git_last_updated_at(
        HOMEWBREW_CASK_REPO,
        cask_paths,
        "Casks",
    )

    formula_entries, formulas, missing_manifests = _collect_formula_entries(
        formulae,
        popularity_by_formula,
        manifests,
        formula_updates,
    )
    cask_entries, cask_metadata = _collect_cask_entries(
        casks.values(),
        popularity_by_cask,
        cask_updates,
    )
    ordered_entries = _sorted_entries(_merge_entries(formula_entries, cask_entries))

    db = {
        "schema": SCHEMA_VERSION,
        "generated_at": datetime.datetime.now(
            datetime.timezone.utc
        ).isoformat(),
        "entries": ordered_entries,
        "formulas": formulas,
        "casks": cask_metadata,
    }

    os.makedirs(os.path.dirname(DB_PATH), exist_ok=True)
    with open(DB_PATH, "w", encoding="utf-8") as handle:
        json.dump(db, handle, indent=2, sort_keys=True)
        handle.write("\n")

    print(
        f"Wrote {DB_PATH} with {len(ordered_entries)} executables "
        f"and {len(formulas)} formulas, {len(cask_metadata)} casks"
    )
    if missing_manifests:
        print(
            f"Skipped {missing_manifests} formulas missing cached manifests",
            file=sys.stderr,
        )


if __name__ == "__main__":
    main()
