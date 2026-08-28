#!/usr/local/bin/av inject -- /bin/bash
# --- automic-vault
# capabilities:
#   gh: write
#   aws: write
# ---
# shellcheck shell=bash disable=SC1008,SC2096
set -euo pipefail

ROOT="$(cd "$(dirname "${AV_SCRIPT_PATH:-$0}")/../.." && pwd)"
SELF="${AV_SCRIPT_PATH:-$0}"
ACTION="${1:-continue}"
VERSION="${2:-}"
REPOSITORY="example/widget"
BUCKET="downloads.example.com"
DISTRIBUTION_ID="E123EXAMPLE"

[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
  echo "usage: $SELF {continue|agent:github-context|agent:cdn-status} VERSION" >&2
  exit 64
}

NOTES="$ROOT/.release/$VERSION/notes.md"
ASSETS=("$ROOT/dist/widget-$VERSION.tar.gz" "$ROOT/dist/SHA256SUMS")

agent_prompt() {
  mkdir -p "$(dirname "$NOTES")"
  cat <<EOF
Write concise release notes for $REPOSITORY $VERSION to:
  $NOTES

If you need GitHub release context, run:
  "$SELF" agent:github-context "$VERSION"

If you need the current S3 and CloudFront status, run:
  "$SELF" agent:cdn-status "$VERSION"

Do not run gh or aws directly. Do not include secrets. When the notes are ready,
continue the deterministic publication with:
  "$SELF" continue "$VERSION"
EOF
}

verify_inputs() {
  [[ -f "$NOTES" && ! -L "$NOTES" && -s "$NOTES" ]] || {
    echo "error: release notes must be a nonempty regular file" >&2
    exit 65
  }
  [[ $(wc -c <"$NOTES") -le 65536 ]] || {
    echo "error: release notes exceed 64 KiB" >&2
    exit 65
  }
  for asset in "${ASSETS[@]}"; do
    [[ -f "$asset" && ! -L "$asset" ]] || {
      echo "error: missing release asset: $asset" >&2
      exit 65
    }
  done
}

verify_github_assets() {
  local asset name expected actual
  for asset in "${ASSETS[@]}"; do
    name="$(basename "$asset")"
    expected="sha256:$(shasum -a 256 "$asset" | awk '{print $1}')"
    actual="$(gh release view "$VERSION" --repo "$REPOSITORY" --json assets \
      --jq ".assets[] | select(.name == \"$name\") | .digest")"
    [[ "$actual" == "$expected" ]] || {
      echo "error: GitHub asset differs: $name" >&2
      exit 1
    }
  done
}

publish_github() {
  local is_draft
  if ! gh release view "$VERSION" --repo "$REPOSITORY" >/dev/null 2>&1; then
    gh release create "$VERSION" "${ASSETS[@]}" --repo "$REPOSITORY" \
      --notes-file "$NOTES" --verify-tag
  fi
  is_draft="$(gh release view "$VERSION" --repo "$REPOSITORY" \
    --json isDraft --jq .isDraft)"
  [[ "$is_draft" == false ]] || {
    echo "error: GitHub release is still a draft" >&2
    exit 1
  }
  verify_github_assets
}

mirror_asset() {
  local asset="$1" name key digest checksum remote
  name="$(basename "$asset")"
  key="releases/$VERSION/$name"
  digest="$(shasum -a 256 "$asset" | awk '{print $1}')"
  [[ "sha256:$digest" == "$(gh release view "$VERSION" --repo "$REPOSITORY" \
    --json assets --jq ".assets[] | select(.name == \"$name\") | .digest")" ]] || {
    echo "error: local asset no longer matches GitHub: $name" >&2
    exit 1
  }
  checksum="$(printf '%s' "$digest" | xxd -r -p | base64)"
  remote="$(aws s3api head-object --bucket "$BUCKET" --key "$key" \
    --checksum-mode ENABLED --query ChecksumSHA256 --output text 2>/dev/null || true)"
  case "$remote" in
    "$checksum") return ;;
    ""|None)
      aws s3api put-object --bucket "$BUCKET" --key "$key" --body "$asset" \
        --checksum-sha256 "$checksum" --if-none-match '*' >/dev/null
      ;;
    *) echo "error: S3 asset differs: $key" >&2; exit 1 ;;
  esac
}

case "$ACTION" in
  agent:github-context)
    gh release list --repo "$REPOSITORY" --limit 5
    git -C "$ROOT" log -10 --oneline
    ;;
  agent:cdn-status)
    aws s3api list-objects-v2 --bucket "$BUCKET" --prefix "releases/$VERSION/"
    aws cloudfront list-invalidations --distribution-id "$DISTRIBUTION_ID" --max-items 5
    ;;
  continue)
    if [[ ! -s "$NOTES" ]]; then
      agent_prompt
      exit 75
    fi
    verify_inputs
    publish_github
    for asset in "${ASSETS[@]}"; do mirror_asset "$asset"; done
    aws cloudfront create-invalidation --distribution-id "$DISTRIBUTION_ID" \
      --invalidation-batch "{\"Paths\":{\"Quantity\":1,\"Items\":[\"/releases/$VERSION/*\"]},\"CallerReference\":\"release-$VERSION\"}" \
      >/dev/null
    ;;
  *) echo "error: unknown action: $ACTION" >&2; exit 64 ;;
esac
