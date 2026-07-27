#!/usr/local/bin/av inject --allow-missing-keys +GH_TOKEN -- /bin/bash
# --- automic-vault
# capabilities:
#   gh: trusted
# ---
# shellcheck shell=bash disable=SC1008,SC2096
set -euo pipefail

if [[ $# -ne 0 ]]; then
  echo "usage: $0" >&2
  exit 64
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPOSITORY="automic-vault/automic-vault"
TAP_ROOT="${AUTOMIC_VAULT_REPO_CACHE:-$ROOT/../isotopes}/homebrew-isotopes"
VERSION="$(
  awk -F '"' '
    /^\[package\]/ { package = 1; next }
    /^\[/ { package = 0 }
    package && /^[[:space:]]*version[[:space:]]*=/ { print $2; exit }
  ' "$ROOT/Cargo.toml"
)"

prepare_cask_publish() {
  local branch origin
  if [[ ! -d "$TAP_ROOT/.git" ]]; then
    echo "error: publish requires the Homebrew tap at $TAP_ROOT" >&2
    exit 64
  fi
  origin="$(git -C "$TAP_ROOT" remote get-url origin)"
  case "$origin" in
    git@github.com:automic-vault/homebrew-isotopes.git | https://github.com/automic-vault/homebrew-isotopes.git) ;;
    *)
      echo "error: unexpected Homebrew tap origin: $origin" >&2
      exit 64
      ;;
  esac
  branch="$(git -C "$TAP_ROOT" branch --show-current)"
  if [[ "$branch" != "main" ]]; then
    echo "error: Homebrew tap must be on main, found ${branch:-detached HEAD}" >&2
    exit 64
  fi
  if [[ -n "$(git -C "$TAP_ROOT" status --porcelain --untracked-files=all)" ]]; then
    echo "error: Homebrew tap must have a clean working tree" >&2
    exit 64
  fi
  git -C "$TAP_ROOT" fetch --quiet origin main
  if [[ "$(git -C "$TAP_ROOT" rev-parse HEAD)" != "$(git -C "$TAP_ROOT" rev-parse origin/main)" ]]; then
    echo "error: Homebrew tap main must match origin/main" >&2
    exit 64
  fi
}

publish_cask() {
  local version="$1"
  local sha256="$2"
  local cask="Casks/automic-vault.rb"
  git -C "$TAP_ROOT" pull --ff-only --quiet origin main
  ruby - "$TAP_ROOT/$cask" "$version" "$sha256" <<'RUBY'
path, version, sha256 = ARGV
contents = File.read(path)
replacements = {
  /^  version "[^"]+"$/ => %(  version "#{version}"),
  /^  sha256 "[0-9a-f]{64}"$/ => %(  sha256 "#{sha256}")
}
replacements.each do |pattern, replacement|
  abort "#{path}: expected exactly one #{pattern.inspect}" unless contents.scan(pattern).one?
  contents.sub!(pattern, replacement)
end
File.write("#{path}.tmp", contents)
File.rename("#{path}.tmp", path)
RUBY
  ruby -c "$TAP_ROOT/$cask"
  git -C "$TAP_ROOT" diff --check -- "$cask"
  if git -C "$TAP_ROOT" diff --quiet -- "$cask"; then
    echo "Homebrew cask is already current."
    return
  fi
  git -C "$TAP_ROOT" add -- "$cask"
  git -C "$TAP_ROOT" commit -m "Update Automic Vault cask to $version"
  git -C "$TAP_ROOT" push origin HEAD:main
}

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "error: version must be in MAJOR.MINOR.PATCH format" >&2
  exit 64
fi
if [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  echo "error: publish dispatches GitHub Actions and must run locally" >&2
  exit 64
fi
if ! command -v gh >/dev/null 2>&1; then
  echo "error: publish requires gh" >&2
  exit 64
fi
if [[ -n "$(git -C "$ROOT" status --porcelain --untracked-files=all)" ]]; then
  echo "error: publish requires a clean checkout" >&2
  exit 64
fi
if [[ "$(git -C "$ROOT" branch --show-current)" != "main" ]]; then
  echo "error: publish requires the main branch" >&2
  exit 64
fi
case "$(git -C "$ROOT" remote get-url origin)" in
  git@github.com:automic-vault/automic-vault.git | https://github.com/automic-vault/automic-vault.git) ;;
  *)
    echo "error: publish requires the automic-vault/automic-vault origin" >&2
    exit 64
    ;;
esac
prepare_cask_publish
git -C "$ROOT" fetch --quiet origin main
head="$(git -C "$ROOT" rev-parse HEAD)"
if [[ "$head" != "$(git -C "$ROOT" rev-parse origin/main)" ]]; then
  echo "error: publish requires main to match origin/main" >&2
  exit 64
fi
if gh release view "$VERSION" --repo "$REPOSITORY" >/dev/null 2>&1 ||
  git -C "$ROOT" ls-remote --exit-code --tags origin "refs/tags/$VERSION" >/dev/null 2>&1; then
  echo "error: release or tag $VERSION already exists; publish a new version" >&2
  exit 64
fi
run_url="$(
  gh workflow run release.yml \
    --repo "$REPOSITORY" \
    --ref main \
    -f version="$VERSION" \
    -f commit="$head"
)"
run_url="${run_url##*$'\n'}"
if [[ ! "$run_url" =~ /actions/runs/([0-9]+)$ ]]; then
  echo "error: could not determine dispatched workflow run from: $run_url" >&2
  exit 1
fi
run_id="${BASH_REMATCH[1]}"
echo "Release workflow: $run_url"
gh run watch "$run_id" --repo "$REPOSITORY" --compact --exit-status
read -r is_draft target_commitish release_url < <(
  gh release view "$VERSION" \
    --repo "$REPOSITORY" \
    --json isDraft,targetCommitish,url \
    --jq '[.isDraft, .targetCommitish, .url] | @tsv'
)
if [[ "$is_draft" != "true" || "$target_commitish" != "$head" ]]; then
  echo "error: workflow did not create the expected draft release" >&2
  exit 1
fi
echo "Draft release ready for review and publication:"
echo "$release_url"
printf "release y/n? "
reply=""
read -r reply || true
if [[ "$reply" != "y" && "$reply" != "Y" ]]; then
  echo "Draft release left unpublished."
  exit 0
fi
gh release edit "$VERSION" \
  --repo "$REPOSITORY" \
  --draft=false \
  --latest
read -r is_draft is_immutable target_commitish < <(
  gh api \
    -H "X-GitHub-Api-Version: 2026-03-10" \
    "repos/$REPOSITORY/releases/tags/$VERSION" \
    --jq '[.draft, .immutable, .target_commitish] | @tsv'
)
if [[ "$is_draft" != "false" || "$is_immutable" != "true" || "$target_commitish" != "$head" ]]; then
  echo "error: published release is not immutable or targets the wrong commit" >&2
  exit 1
fi
digest="$(
  gh release view "$VERSION" \
    --repo "$REPOSITORY" \
    --json assets \
    --jq ".assets[] | select(.name == \"Automic-Vault-$VERSION.dmg\") | .digest"
)"
if [[ ! "$digest" =~ ^sha256:([0-9a-f]{64})$ ]]; then
  echo "error: release DMG has no valid SHA-256 digest" >&2
  exit 1
fi
publish_cask "$VERSION" "${BASH_REMATCH[1]}"
echo "Published release: $release_url"
