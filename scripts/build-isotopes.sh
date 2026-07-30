#!/usr/bin/env bash
set -euo pipefail

org="automic-vault"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
clone_root="${AUTOMIC_VAULT_REPO_CACHE:-${repo_root}/../isotopes}"
only_repo=""
continue_tag=""
dry_run=false
continue_update=false

usage() {
  cat <<'EOF'
Usage: scripts/build-isotopes.sh [--clone-root PATH] [--repo NAME] [--continue --tag TAG] [--dry-run]

For each automic-vault fork, check the latest upstream GitHub release. If the
fork does not already have a release for that tag, rebase the fork's mirrored
upstream default branch onto the upstream tag, then stop with instructions for
the controlling agent. After the agent resolves conflicts and verifies the fork
goals, resume with --continue --repo NAME --tag TAG to build and publish
cli-<version>.tgz to the fork release.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --clone-root)
      clone_root="$2"
      shift 2
      ;;
    --repo)
      only_repo="$2"
      shift 2
      ;;
    --tag)
      continue_tag="$2"
      shift 2
      ;;
    --dry-run)
      dry_run=true
      shift
      ;;
    --continue)
      continue_update=true
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 64
      ;;
  esac
done

if [[ "$continue_update" == true && ( -z "$only_repo" || -z "$continue_tag" ) ]]; then
  echo "--continue requires --repo NAME and --tag TAG" >&2
  exit 64
fi
if [[ "$continue_update" == false && -n "$continue_tag" ]]; then
  echo "--tag requires --continue" >&2
  exit 64
fi

for tool in gh git jq ruby; do
  command -v "$tool" >/dev/null || {
    echo "Missing required tool: $tool" >&2
    exit 1
  }
done

mkdir -p "$clone_root"

sanitize_version() {
  local version="$1"
  version="${version#refs/tags/}"
  version="${version#v}"
  version="${version//\//-}"
  version="${version// /-}"
  printf '%s\n' "$version"
}

manifest_json() {
  ruby -ryaml -rjson -e '
    puts JSON.generate(YAML.safe_load(File.read(ARGV.fetch(0)), aliases: false) || {})
  ' "$1"
}

manifest_field() {
  manifest_json "$1" | jq -r --arg field "$2" '.[$field] // empty'
}

ensure_codesign_identity() {
  if [[ -n "${CODESIGN_IDENTITY:-}" ]]; then
    return 0
  fi
  CODESIGN_IDENTITY="$(
    security find-identity -v -p codesigning 2>/dev/null |
      awk -F '"' '/Developer ID Application/ { print $2; exit }'
  )"
  if [[ -z "$CODESIGN_IDENTITY" ]]; then
    CODESIGN_IDENTITY="$(
      security find-identity -v -p codesigning 2>/dev/null |
        awk -F '"' '/Apple Development/ { print $2; exit }'
    )"
  fi
  if [[ -z "$CODESIGN_IDENTITY" ]]; then
    echo "Missing CODESIGN_IDENTITY and no Apple signing identity was found" >&2
    return 1
  fi
  export CODESIGN_IDENTITY
}

ensure_clone() {
  local repo_name="$1"
  local repo_dir="$clone_root/$repo_name"
  local origin_url="https://github.com/$org/$repo_name.git"

  if [[ "$(gh config get git_protocol --host github.com 2>/dev/null || true)" == ssh ]]; then
    origin_url="git@github.com:$org/$repo_name.git"
  fi

  if [[ -d "$repo_dir/.git" ]]; then
    git -C "$repo_dir" remote set-url origin "$origin_url"
    return 0
  fi
  if [[ -e "$repo_dir" ]]; then
    echo "Clone path exists but is not a git repo: $repo_dir" >&2
    return 1
  fi

  echo "Cloning $org/$repo_name"
  if [[ "$dry_run" == true ]]; then
    echo "Would clone $org/$repo_name to $repo_dir"
  else
    gh repo clone "$org/$repo_name" "$repo_dir"
    git -C "$repo_dir" remote set-url origin "$origin_url"
  fi
}

ensure_fork_branch() {
  local repo_name="$1"
  local branch="$2"
  local current_default="$3"
  local repo_dir="$clone_root/$repo_name"
  local current_branch

  if [[ "$dry_run" == true ]]; then
    echo "Would ensure the mirrored branch is $branch"
    return 0
  fi

  git -C "$repo_dir" fetch --no-tags origin
  current_branch="$(git -C "$repo_dir" branch --show-current || true)"
  if [[ "$current_branch" == "$branch" ]] &&
    git -C "$repo_dir" show-ref --verify --quiet "refs/remotes/origin/$branch" &&
    git -C "$repo_dir" merge-base --is-ancestor "origin/$branch" HEAD &&
    git -C "$repo_dir" diff --quiet &&
    git -C "$repo_dir" diff --cached --quiet &&
    ! git_rebase_in_progress "$repo_dir"; then
    return 0
  fi

  if git -C "$repo_dir" show-ref --verify --quiet "refs/remotes/origin/$branch"; then
    git -C "$repo_dir" checkout -B "$branch" "origin/$branch"
  elif git -C "$repo_dir" show-ref --verify --quiet "refs/remotes/origin/$current_default"; then
    git -C "$repo_dir" push origin "refs/remotes/origin/$current_default:refs/heads/$branch"
    git -C "$repo_dir" fetch --no-tags origin "refs/heads/$branch:refs/remotes/origin/$branch"
    git -C "$repo_dir" checkout -B "$branch" "origin/$branch"
  else
    git -C "$repo_dir" checkout "$(git -C "$repo_dir" branch --show-current)"
    git -C "$repo_dir" branch -M "$branch"
    git -C "$repo_dir" push origin "HEAD:$branch"
  fi
}

set_upstream_remote() {
  local repo_dir="$1"
  local upstream_repo="$2"
  local upstream_url="https://github.com/$upstream_repo.git"

  if git -C "$repo_dir" remote get-url upstream >/dev/null 2>&1; then
    git -C "$repo_dir" remote set-url upstream "$upstream_url"
  else
    git -C "$repo_dir" remote add upstream "$upstream_url"
  fi
}

release_exists() {
  gh release view "$2" --repo "$1" >/dev/null 2>&1
}

latest_release_json() {
  gh api -H "Accept: application/vnd.github+json" "/repos/$1/releases/latest"
}

handoff_to_agent() {
  local repo_dir="$1"
  local fork_repo="$2"
  local upstream_repo="$3"
  local tag="$4"
  local rebase_status="$5"
  local resume_command

  printf -v resume_command '%q --clone-root %q --repo %q --continue --tag %q' \
    "$script_dir/build-isotopes.sh" "$clone_root" "${fork_repo#*/}" "$tag"

  cat >&2 <<EOF
CONTROLLING AGENT ACTION REQUIRED

Verify and, if needed, finish this Automic Vault isotope update:

Fork checkout: $repo_dir
Fork repo: $fork_repo
Upstream repo: $upstream_repo
Upstream release tag: $tag
Rebase exit status: $rebase_status

Work in the fork checkout. If a rebase is in progress, resolve conflicts and
finish it. Then read automic-vault.yml, verify the fork goal is still intact on
top of upstream $tag, and make the smallest fixes needed if upstream changed.
Run the manifest build or the narrowest practical check. Leave the checkout on
the mirrored default branch with no unmerged paths, no rebase/merge/cherry-pick
in progress, and no uncommitted changes.

Then resume the release with:
  $resume_command
EOF
}

git_clean() {
  local repo_dir="$1"
  local rebase_apply rebase_merge

  rebase_apply="$(git -C "$repo_dir" rev-parse --path-format=absolute --git-path rebase-apply)"
  rebase_merge="$(git -C "$repo_dir" rev-parse --path-format=absolute --git-path rebase-merge)"

  git -C "$repo_dir" diff --quiet &&
    git -C "$repo_dir" diff --cached --quiet &&
    ! git -C "$repo_dir" diff --name-only --diff-filter=U | grep -q . &&
    [[ ! -d "$rebase_apply" ]] &&
    [[ ! -d "$rebase_merge" ]] &&
    ! git -C "$repo_dir" rev-parse -q --verify MERGE_HEAD >/dev/null 2>&1 &&
    ! git -C "$repo_dir" rev-parse -q --verify CHERRY_PICK_HEAD >/dev/null 2>&1 &&
    ! git -C "$repo_dir" rev-parse -q --verify REVERT_HEAD >/dev/null 2>&1
}

git_rebase_in_progress() {
  local repo_dir="$1"
  local rebase_apply rebase_merge

  rebase_apply="$(git -C "$repo_dir" rev-parse --path-format=absolute --git-path rebase-apply)"
  rebase_merge="$(git -C "$repo_dir" rev-parse --path-format=absolute --git-path rebase-merge)"
  [[ -d "$rebase_apply" || -d "$rebase_merge" ]]
}

build_manifest() {
  local repo_dir="$1"
  local tag="$2"
  local version="$3"
  local manifest_path="$repo_dir/automic-vault.yml"
  local build_script

  build_script="$(manifest_field "$manifest_path" build)"
  if [[ -z "$build_script" ]]; then
    echo "Missing build in $manifest_path" >&2
    return 1
  fi

  ensure_codesign_identity
  (
    cd "$repo_dir"
    CI="${CI:-true}" TAG="$tag" VERSION="$version" bash -euo pipefail -c "$build_script"
  )
}

find_output() {
  local repo_dir="$1"
  local repo_name="$2"

  if [[ -f "$repo_dir/isotopes/$repo_name/out.tgz" ]]; then
    printf '%s\n' "$repo_dir/isotopes/$repo_name/out.tgz"
  elif [[ -f "$repo_dir/out.tgz" ]]; then
    printf '%s\n' "$repo_dir/out.tgz"
  else
    return 1
  fi
}

verify_archive_signatures() {
  local archive_path="$1"
  local archive_dir file found=false

  archive_dir="$(mktemp -d "${TMPDIR:-/tmp}/automic-vault-isotope.XXXXXX")"
  tar -tzf "$archive_path" | awk '
    $0 !~ /^bin\// || $0 ~ /(^|\/)\.\.?(\/|$)/ { exit 1 }
  ' || {
    echo "Release archive must contain only safe bin/ paths: $archive_path" >&2
    return 1
  }
  tar -xzf "$archive_path" -C "$archive_dir"
  while IFS= read -r -d '' file; do
    found=true
    codesign --verify --strict --verbose=2 "$file"
  done < <(find "$archive_dir/bin" -type f -perm -111 -print0)
  if [[ "$found" == false ]]; then
    echo "Release archive contains no executable to verify: $archive_path" >&2
    rm -rf "$archive_dir"
    return 1
  fi
  rm -rf "$archive_dir"
}

process_repo() {
  local repo_name="$1"
  local fork_repo="$org/$repo_name"
  local repo_dir="$clone_root/$repo_name"
  local repo_json upstream_repo upstream_default current_default release_json tag version release_url output archive_path status

  echo "Checking $fork_repo"
  ensure_clone "$repo_name"

  repo_json="$(gh api "/repos/$fork_repo")"
  upstream_repo="$(jq -r '.parent.full_name // empty' <<<"$repo_json")"
  if [[ -z "$upstream_repo" ]]; then
    echo "Skipping $fork_repo: not a GitHub fork"
    return 0
  fi
  upstream_default="$(jq -r '.parent.default_branch // empty' <<<"$repo_json")"
  current_default="$(jq -r '.default_branch // empty' <<<"$repo_json")"
  if [[ -z "$upstream_default" ]]; then
    echo "Skipping $fork_repo: upstream default branch is unavailable"
    return 0
  fi
  if [[ "$continue_update" == false ]]; then
    ensure_fork_branch "$repo_name" "$upstream_default" "$current_default"
  fi

  if [[ "$continue_update" == true ]]; then
    release_json="$(gh api -H "Accept: application/vnd.github+json" \
      "/repos/$upstream_repo/releases/tags/$continue_tag")"
  else
    release_json="$(latest_release_json "$upstream_repo")"
  fi
  tag="$(jq -r '.tag_name' <<<"$release_json")"
  release_url="$(jq -r '.html_url' <<<"$release_json")"
  if [[ -z "$tag" || "$tag" == null ]]; then
    echo "Skipping $fork_repo: upstream has no latest release tag"
    return 0
  fi

  if release_exists "$fork_repo" "$tag"; then
    echo "Skipping $fork_repo: release $tag already exists"
    return 0
  fi

  version="$(sanitize_version "$tag")"
  archive_path="$repo_dir/cli-$version.tgz"
  echo "New upstream release for $fork_repo: $upstream_repo $tag"

  if [[ "$dry_run" == true ]]; then
    if [[ "$continue_update" == true ]]; then
      echo "Would validate the repaired checkout, build, and release $archive_path"
    else
      echo "Would rebase $upstream_default onto upstream tag $tag and stop for the controlling agent"
    fi
    return 0
  fi

  set_upstream_remote "$repo_dir" "$upstream_repo"
  git -C "$repo_dir" fetch --no-tags upstream "+refs/tags/$tag:refs/tags/$tag"

  if [[ "$continue_update" == false ]]; then
    set +e
    git -C "$repo_dir" rebase "refs/tags/$tag"
    status=$?
    set -e
    handoff_to_agent "$repo_dir" "$fork_repo" "$upstream_repo" "$tag" "$status"
    exit 75
  fi

  if [[ "$(git -C "$repo_dir" branch --show-current)" != "$upstream_default" ]]; then
    echo "Cannot continue $fork_repo: checkout must be on $upstream_default" >&2
    return 1
  fi
  if ! git_clean "$repo_dir"; then
    echo "Cannot continue $fork_repo: checkout is not clean" >&2
    git -C "$repo_dir" status --short >&2
    return 1
  fi
  if ! git -C "$repo_dir" merge-base --is-ancestor "refs/tags/$tag" HEAD; then
    echo "Cannot continue $fork_repo: HEAD is not based on upstream tag $tag" >&2
    return 1
  fi

  git -C "$repo_dir" tag -f "$tag" HEAD
  build_manifest "$repo_dir" "$tag" "$version"
  output="$(find_output "$repo_dir" "$repo_name")"
  mv -f "$output" "$archive_path"
  verify_archive_signatures "$archive_path"

  git -C "$repo_dir" push origin "HEAD:$upstream_default" --force-with-lease
  git -C "$repo_dir" push origin "+refs/tags/$tag:refs/tags/$tag"
  gh release create "$tag" "$archive_path" \
    --repo "$fork_repo" \
    --title "$tag" \
    --verify-tag \
    --notes "Built from $upstream_repo $tag: $release_url"
}

repo_names="$(
  gh repo list "$org" --limit 1000 --json isFork,name \
    --jq '.[] | select(.isFork) | .name'
)"
if [[ -n "$only_repo" ]]; then
  repo_names="$(awk -v repo="$only_repo" '$0 == repo' <<<"$repo_names")"
fi
if [[ -z "$repo_names" ]]; then
  echo "No repositories found for $org" >&2
  exit 1
fi

while IFS= read -r repo_name; do
  [[ -n "$repo_name" ]] || continue
  process_repo "$repo_name"
done <<<"$repo_names"
