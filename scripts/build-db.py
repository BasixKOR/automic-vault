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
NPM_ECOSYSTEM = "npmjs"
DB_PATH = os.path.join("data", "db.json")
SCHEMA_VERSION = 7
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
NPM_REGISTRY_ROOT = "https://registry.npmjs.org"
NPM_REPLICATE_CHANGES_URL = "https://replicate.npmjs.com/registry/_changes"
NPM_SEARCH_URL = "https://registry.npmjs.org/-/v1/search"
NPM_DOWNLOADS_POINT_ROOT = "https://api.npmjs.org/downloads/point/last-month"
NPM_MIN_MONTHLY_DOWNLOADS = 50_000
NPM_SEARCH_PAGE_SIZE = 250
NPM_SEARCH_MAX_PAGES = 4
PULSE_NEW_WINDOW_DAYS = 7
NPM_CHANGES_LIMIT = 5000
NPM_INDEX_STATE_PATH = os.path.join(CACHE_DIR, NPM_ECOSYSTEM, "index.json")
NPM_SEARCH_QUERIES = (
    "cli",
    "command",
    "command-line",
    "terminal",
    "shell",
    "devtool",
    "runner",
    "linter",
    "formatter",
    "generator",
)
NPM_SEED_PACKAGES = (
    "tsx",
    "vite",
    "eslint",
    "prettier",
    "serve",
    "nodemon",
    "vitest",
    "webpack-cli",
    "rollup",
)

_GHCR_TOKENS = {}


def _ensure_cwd():
    scripts_dir = os.path.abspath(os.path.dirname(__file__))
    root = os.path.dirname(scripts_dir)
    os.chdir(root)


def _cache_path_for(url, ecosystem):
    digest = hashlib.sha256(url.encode("utf-8")).hexdigest()
    return os.path.join(CACHE_DIR, ecosystem, f"{digest}.json")


def _cache_path(url):
    return _cache_path_for(url, ECOSYSTEM)


def _read_cached_json(url, ecosystem=ECOSYSTEM):
    path = _cache_path_for(url, ecosystem)
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


def _fetch_json(url, github_token=None, ecosystem=ECOSYSTEM, accept="application/json"):
    path = _cache_path_for(url, ecosystem)
    payload = None
    meta = {}
    if os.path.exists(path):
        payload, meta = _read_cached_json(url, ecosystem)

    checked_at = meta.get("checked_at")
    now = int(time.time())
    if (
        not FORCE_REFRESH
        and isinstance(checked_at, int)
        and now - checked_at < CHECK_INTERVAL_SECONDS
    ):
        return payload

    headers = {"Accept": accept, "User-Agent": USER_AGENT}
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
    return None


def _github_username():
    for key in ("GHCR_USERNAME", "GITHUB_ACTOR", "USER"):
        value = os.environ.get(key)
        if value:
            return value.strip()
    return None


def _ghcr_repo_from_url(path):
    parts = [part for part in path.split("/") if part]
    if len(parts) < 4 or parts[0] != "v2":
        return None
    return "/".join(parts[1:-2])


def _ghcr_bearer_token(repo, github_token):
    now = int(time.time())
    cache_key = (repo, bool(github_token))
    cached = _GHCR_TOKENS.get(cache_key)
    if cached and cached["expires_at"] > now:
        return cached["token"]

    username = _github_username() or "x-access-token"
    scope = f"repository:{repo}:pull"
    query = urllib.parse.urlencode({"service": "ghcr.io", "scope": scope})
    url = f"{TOKEN_SERVICE}?{query}"
    headers = {"User-Agent": USER_AGENT}
    if github_token:
        basic = base64.b64encode(
            f"{username}:{github_token}".encode("utf-8")
        ).decode("utf-8")
        headers["Authorization"] = f"Basic {basic}"
    request = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=DEFAULT_TIMEOUT) as response:
            data = json.loads(response.read())
    except urllib.error.HTTPError as err:
        if github_token:
            return _ghcr_bearer_token(repo, None)
        print(f"Failed to get GHCR token for {repo}: {err}", file=sys.stderr)
        return None
    bearer = data.get("token")
    expires_in = data.get("expires_in", 300)
    if bearer:
        _GHCR_TOKENS[cache_key] = {
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


def _git_pulse_events(repo, keyed_paths, scope):
    if not keyed_paths:
        return {}

    repo_path = _ensure_git_repo(repo)
    revision = _git_default_revision(repo_path)
    new_cutoff = datetime.datetime.now(
        datetime.timezone.utc
    ) - datetime.timedelta(days=PULSE_NEW_WINDOW_DAYS)
    pending_latest = set(keyed_paths.keys())
    pending_additions = set(keyed_paths.keys())
    events = {}
    current_date = None
    current_datetime = None
    recent_additions = set()
    command = [
        "git",
        "-C",
        repo_path,
        "log",
        revision,
        "--format=__DATE__%cI",
        "--name-status",
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
                current_datetime = _parse_git_timestamp(current_date)
                continue
            if current_date is None:
                continue
            parts = line.split("\t")
            if len(parts) < 2:
                continue
            status = parts[0]
            path = parts[-1]
            if path not in keyed_paths:
                continue
            key = keyed_paths[path]
            added_latest = False
            if path in pending_latest:
                events[key] = {
                    "last_updated_at": current_date,
                    "pulse_kind": "updated",
                }
                pending_latest.remove(path)
                added_latest = True
            if path in pending_additions:
                if status.startswith("A") and _is_recent_datetime(
                    current_datetime,
                    new_cutoff,
                ):
                    recent_additions.add(key)
                if status.startswith("A") or not _is_recent_datetime(
                    current_datetime,
                    new_cutoff,
                ):
                    pending_additions.remove(path)
            if added_latest and len(events) % 100 == 0:
                print(
                    f"Resolved {len(events)}/{len(keyed_paths)} git pulse events for {repo}",
                    file=sys.stderr,
                )
            if not pending_latest and not _is_recent_datetime(
                current_datetime,
                new_cutoff,
            ):
                process.terminate()
                break
    finally:
        stdout, stderr = process.communicate()
        if process.returncode not in (0, -15):
            message = stderr.strip() or stdout.strip() or f"git log failed for {repo}"
            raise RuntimeError(message)

    for key in recent_additions:
        if key in events:
            events[key]["pulse_kind"] = "new"
    return events


def _parse_git_timestamp(value):
    try:
        return datetime.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None


def _is_recent_datetime(value, cutoff):
    return value is not None and value >= cutoff


def _parse_binary_artifact(artifact):
    if not isinstance(artifact, dict):
        return None
    if "binary" not in artifact or not set(artifact.keys()) <= {"binary", "target"}:
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

    if target is None:
        artifact_target = artifact.get("target")
        if isinstance(artifact_target, str) and artifact_target:
            target = os.path.basename(artifact_target)

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
        if (
            isinstance(artifact, dict)
            and set(artifact.keys())
            <= {"generate_completions_from_executable", "zap", "uninstall"}
        ):
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


def _collect_formula_entries(formulae, popularity_by_formula, manifests, pulse_events_by_formula):
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
        pulse_event = pulse_events_by_formula.get(name)
        if name in formulas:
            if popularity is not None:
                formulas[name]["popularity"] = popularity
            if pulse_event is not None:
                formulas[name]["last_updated_at"] = pulse_event["last_updated_at"]
                formulas[name]["pulse_kind"] = pulse_event["pulse_kind"]

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


def _collect_cask_entries(casks, popularity_by_cask, pulse_events_by_cask):
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
        pulse_event = pulse_events_by_cask.get(token)
        if supported["binaries"] and pulse_event is not None:
            supported["last_updated_at"] = pulse_event["last_updated_at"]
            supported["pulse_kind"] = pulse_event["pulse_kind"]
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


def _fetch_uncached_json(url, accept="application/json"):
    request = urllib.request.Request(
        url,
        headers={"Accept": accept, "User-Agent": USER_AGENT},
    )
    with urllib.request.urlopen(request, timeout=DEFAULT_TIMEOUT) as response:
        return json.loads(response.read())


def _npm_package_url(package):
    return f"{NPM_REGISTRY_ROOT}/{urllib.parse.quote(package, safe='@')}"


def _npm_downloads_url(package):
    return f"{NPM_DOWNLOADS_POINT_ROOT}/{urllib.parse.quote(package, safe='@/')}"


def _npm_search_url(query, offset):
    params = urllib.parse.urlencode(
        {
            "text": query,
            "size": NPM_SEARCH_PAGE_SIZE,
            "from": offset,
            "quality": 0,
            "maintenance": 0,
            "popularity": 1,
        }
    )
    return f"{NPM_SEARCH_URL}?{params}"


def _npm_install_leaf_name(package):
    if package.startswith("@") and "/" in package:
        return package.rsplit("/", 1)[-1]
    return package


def _npm_matching_executable(package, bin_value):
    leaf = _npm_install_leaf_name(package)
    if isinstance(bin_value, str) and bin_value:
        return leaf
    if not isinstance(bin_value, dict):
        return None
    target = bin_value.get(leaf)
    if isinstance(target, str) and target:
        return leaf
    return None


def _npm_latest_version_doc(packument):
    if not isinstance(packument, dict):
        return None, None
    latest = (packument.get("dist-tags") or {}).get("latest")
    versions = packument.get("versions") or {}
    if not isinstance(latest, str) or not isinstance(versions, dict):
        return None, None
    version_doc = versions.get(latest)
    if not isinstance(version_doc, dict):
        return None, None
    return latest, version_doc


def _npm_last_updated_at(packument, latest):
    times = packument.get("time") if isinstance(packument, dict) else None
    if not isinstance(times, dict):
        return None
    value = times.get(latest) or times.get("modified")
    return value if isinstance(value, str) and value else None


def _npm_metadata_from_packument(package, packument, monthly_downloads):
    if monthly_downloads < NPM_MIN_MONTHLY_DOWNLOADS:
        return None
    latest, version_doc = _npm_latest_version_doc(packument)
    if latest is None or version_doc is None:
        return None
    if version_doc.get("deprecated"):
        return None
    executable = _npm_matching_executable(package, version_doc.get("bin"))
    if executable is None:
        return None

    summary = version_doc.get("description") or packument.get("description") or ""
    homepage = version_doc.get("homepage") or packument.get("homepage") or ""
    return {
        "summary": summary if isinstance(summary, str) else "",
        "homepage": homepage if isinstance(homepage, str) else "",
        "version": latest,
        "executable": executable,
        "popularity": {
            "downloads_per_30_days": monthly_downloads,
            "rank": 0,
        },
        "last_updated_at": _npm_last_updated_at(packument, latest),
    }


def _fetch_npm_packument(package):
    return _fetch_json(
        _npm_package_url(package),
        ecosystem=NPM_ECOSYSTEM,
        accept="application/json",
    )


def _fetch_npm_monthly_downloads(package):
    payload = _fetch_json(_npm_downloads_url(package), ecosystem=NPM_ECOSYSTEM)
    if not isinstance(payload, dict):
        return 0
    return _parse_count(payload.get("downloads")) or 0


def _fetch_npm_search_candidates():
    candidates = {}
    for package in NPM_SEED_PACKAGES:
        try:
            monthly = _fetch_npm_monthly_downloads(package)
        except Exception as err:
            print(f"Skipping seeded npm package {package}: {err}", file=sys.stderr)
            continue
        if monthly >= NPM_MIN_MONTHLY_DOWNLOADS:
            candidates[package] = monthly

    for query in NPM_SEARCH_QUERIES:
        for page in range(NPM_SEARCH_MAX_PAGES):
            url = _npm_search_url(query, page * NPM_SEARCH_PAGE_SIZE)
            try:
                payload = _fetch_json(url, ecosystem=NPM_ECOSYSTEM)
            except urllib.error.HTTPError as err:
                if err.code in (429, 503):
                    print(
                        f"Skipping npm search page for {query!r}: {err}",
                        file=sys.stderr,
                    )
                    break
                raise
            except urllib.error.URLError as err:
                print(
                    f"Skipping npm search page for {query!r}: {err}",
                    file=sys.stderr,
                )
                break
            objects = payload.get("objects") if isinstance(payload, dict) else None
            if not objects:
                break
            for item in objects:
                package = item.get("package") if isinstance(item, dict) else None
                downloads = item.get("downloads") if isinstance(item, dict) else None
                if not isinstance(package, dict) or not isinstance(downloads, dict):
                    continue
                name = package.get("name")
                monthly = _parse_count(downloads.get("monthly")) or 0
                if not isinstance(name, str) or not name:
                    continue
                if monthly < NPM_MIN_MONTHLY_DOWNLOADS:
                    continue
                current = candidates.get(name)
                if current is None or monthly > current:
                    candidates[name] = monthly
            if len(objects) < NPM_SEARCH_PAGE_SIZE:
                break
    return candidates


def _read_npm_index_state():
    if not os.path.exists(NPM_INDEX_STATE_PATH):
        return {"last_seq": None, "packages": {}}
    with open(NPM_INDEX_STATE_PATH, "r", encoding="utf-8") as handle:
        payload = json.load(handle)
    if not isinstance(payload, dict):
        return {"last_seq": None, "packages": {}}
    packages = payload.get("packages")
    if not isinstance(packages, dict):
        packages = {}
    return {
        "last_seq": payload.get("last_seq"),
        "packages": packages,
    }


def _write_npm_index_state(last_seq, packages):
    os.makedirs(os.path.dirname(NPM_INDEX_STATE_PATH), exist_ok=True)
    with open(NPM_INDEX_STATE_PATH, "w", encoding="utf-8") as handle:
        json.dump(
            {
                "last_seq": last_seq,
                "packages": packages,
            },
            handle,
            indent=2,
            sort_keys=True,
        )
        handle.write("\n")


def _current_npm_changes_sequence():
    params = urllib.parse.urlencode({"descending": "true", "limit": 1})
    payload = _fetch_uncached_json(f"{NPM_REPLICATE_CHANGES_URL}?{params}")
    return payload.get("last_seq") if isinstance(payload, dict) else None


def _fetch_npm_changes_since(last_seq):
    if last_seq is None:
        return set(), set(), _current_npm_changes_sequence()

    changed = set()
    deleted = set()
    next_seq = last_seq
    while True:
        params = urllib.parse.urlencode({"since": next_seq, "limit": NPM_CHANGES_LIMIT})
        payload = _fetch_uncached_json(f"{NPM_REPLICATE_CHANGES_URL}?{params}")
        if not isinstance(payload, dict):
            break
        results = payload.get("results")
        if not isinstance(results, list) or not results:
            next_seq = payload.get("last_seq", next_seq)
            break
        for item in results:
            if not isinstance(item, dict):
                continue
            package = item.get("id")
            if not isinstance(package, str) or not package:
                continue
            if item.get("deleted"):
                deleted.add(package)
                changed.discard(package)
            else:
                changed.add(package)
        next_seq = payload.get("last_seq", next_seq)
        if len(results) < NPM_CHANGES_LIMIT:
            break
    return changed, deleted, next_seq


def _collect_npm_metadata():
    state = _read_npm_index_state()
    packages = {
        name: metadata
        for name, metadata in state["packages"].items()
        if isinstance(name, str) and isinstance(metadata, dict)
    }

    candidates = _fetch_npm_search_candidates()
    changed, deleted, next_seq = _fetch_npm_changes_since(state.get("last_seq"))
    for package in deleted:
        packages.pop(package, None)

    refresh_names = set(candidates)
    refresh_names.update(package for package in changed if package in packages)
    if changed:
        print(
            f"Processing {len(refresh_names)} npm candidates from "
            f"{len(changed)} registry changes...",
            file=sys.stderr,
        )

    completed = 0
    max_workers = min(32, (os.cpu_count() or 4) * 4)

    def refresh(package):
        try:
            monthly = candidates.get(package)
            if monthly is None:
                monthly = _fetch_npm_monthly_downloads(package)
            packument = _fetch_npm_packument(package)
            if packument is None:
                return package, None
            return package, _npm_metadata_from_packument(package, packument, monthly)
        except Exception as err:
            print(f"Failed to refresh npm package {package}: {err}", file=sys.stderr)
            return package, packages.get(package)

    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_map = {
            executor.submit(refresh, package): package
            for package in sorted(refresh_names)
        }
        for future in as_completed(future_map):
            package, metadata = future.result()
            if metadata is None:
                packages.pop(package, None)
            else:
                packages[package] = metadata
            completed += 1
            if completed % 100 == 0:
                print(
                    f"Refreshed {completed}/{len(refresh_names)} npm packages...",
                    file=sys.stderr,
                )

    ranked = {}
    sorted_packages = sorted(
        packages.items(),
        key=lambda item: (
            -((item[1].get("popularity") or {}).get("downloads_per_30_days") or 0),
            item[0],
        ),
    )
    for rank, (package, metadata) in enumerate(sorted_packages, start=1):
        popularity = metadata.setdefault("popularity", {})
        popularity["rank"] = rank
        ranked[package] = metadata

    if next_seq is not None:
        _write_npm_index_state(next_seq, ranked)
    return ranked


def _apply_npm_entries(ordered_entries, npm_metadata):
    candidates = sorted(
        npm_metadata.items(),
        key=lambda item: (
            -((item[1].get("popularity") or {}).get("downloads_per_30_days") or 0),
            item[0],
        ),
    )
    for package, metadata in candidates:
        executable = metadata.get("executable")
        if isinstance(executable, str) and executable:
            ordered_entries.setdefault(executable, f"npm:{package}")
    return ordered_entries


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
    os.makedirs(os.path.join(CACHE_DIR, NPM_ECOSYSTEM), exist_ok=True)

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
            completed += 1
            if completed % 20 == 0:
                print(
                    "Fetched "
                    f"{completed}/"
                    f"{len(manifest_urls) + len(cask_urls)} "
                    "api payloads...",
                    file=sys.stderr,
                )

    formula_pulse_events = _git_pulse_events(
        HOMEWBREW_CORE_REPO,
        formula_paths,
        "Formula",
    )
    cask_pulse_events = _git_pulse_events(
        HOMEWBREW_CASK_REPO,
        cask_paths,
        "Casks",
    )

    formula_entries, formulas, missing_manifests = _collect_formula_entries(
        formulae,
        popularity_by_formula,
        manifests,
        formula_pulse_events,
    )
    cask_entries, cask_metadata = _collect_cask_entries(
        casks.values(),
        popularity_by_cask,
        cask_pulse_events,
    )
    if cask_urls and not cask_metadata:
        print(
            "No supported cask metadata was collected; refusing to write a "
            "database without casks.",
            file=sys.stderr,
        )
        sys.exit(2)
    npm_metadata = _collect_npm_metadata()
    ordered_entries = _sorted_entries(_merge_entries(formula_entries, cask_entries))
    ordered_entries = _apply_npm_entries(ordered_entries, npm_metadata)

    db = {
        "schema": SCHEMA_VERSION,
        "generated_at": datetime.datetime.now(
            datetime.timezone.utc
        ).isoformat(),
        "entries": ordered_entries,
        "formulas": formulas,
        "casks": cask_metadata,
        "npms": npm_metadata,
    }

    os.makedirs(os.path.dirname(DB_PATH), exist_ok=True)
    with open(DB_PATH, "w", encoding="utf-8") as handle:
        json.dump(db, handle, indent=2, sort_keys=True)
        handle.write("\n")

    print(
        f"Wrote {DB_PATH} with {len(ordered_entries)} executables "
        f"and {len(formulas)} formulas, {len(cask_metadata)} casks, "
        f"{len(npm_metadata)} npm packages"
    )
    if missing_manifests:
        print(
            f"Skipped {missing_manifests} formulas missing cached manifests",
            file=sys.stderr,
        )


if __name__ == "__main__":
    main()
