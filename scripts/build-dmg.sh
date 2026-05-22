#!/usr/local/bin/av inject +APPLE_PASSWORD /usr/local/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
source "${repo_root}/scripts/cli-style.sh"
cli_style_init "Automic Vault"
build_dir="${repo_root}/target/gui"
target_dir="${repo_root}/target"
default_background="${repo_root}/assets/dmg-bg@2x.png"
release_s3_uri="s3://automicvault.com/Automic Vault.dmg"
release_cloudfront_alias="${AUTOMIC_VAULT_RELEASE_DOMAIN:-automicvault.com}"
release_cloudfront_path="/Automic%20Vault.dmg"
finder_left=120
finder_top=120
finder_width=796
finder_height=494
icon_size=128
app_icon_x=243
applications_icon_x=553

output_path=""
background_path=""
volume_name=""
prepared_background_path=""
notarize=false
install_app=false
publish_release=false
clobber_release=false

usage() {
  cat <<'EOF'
Usage: scripts/build-dmg.sh [--output PATH] [--background PATH]
                            [--volume-name NAME] [--notarize] [--install]
                            [--publish] [--clobber]

Build the release app bundle and package it into a DMG in target/.

Options:
  --output PATH       Write the final DMG to PATH.
  --background PATH   Use PATH as the Finder window background image.
  --volume-name NAME  Override the mounted DMG volume name.
  --notarize          Submit the DMG for notarization and staple it.
  --notorize          Alias for --notarize.
  --install           Install the built app bundle into /Applications.
  --publish           Create a GitHub release for vX.Y.Z with the DMG.
                      Requires --notarize.
  --clobber           Delete any existing GitHub release for vX.Y.Z before
                      publishing. Requires --publish.
  --help              Show this help.
EOF
}

publish_github_release() {
  local tag="$1"
  local version="$2"
  local dmg_path="$3"
  local asset_label
  local release_notes_path
  local target_ref
  local -a release_args

  asset_label="$(basename "${dmg_path}")"
  target_ref="$(git -C "${repo_root}" rev-parse --abbrev-ref HEAD)"

  if [[ "${target_ref}" == "HEAD" ]]; then
    target_ref="$(git -C "${repo_root}" rev-parse HEAD)"
  fi

  if [[ "${clobber_release}" == "true" ]]; then
    release_notes_path="$(clobber_github_release "${tag}")"
  else
    release_notes_path="$(generate_release_notes "${tag}" "${target_ref}")"
  fi

  release_args=(
    "${tag}"
    --draft
    --notes-file "${release_notes_path}"
    --target "${target_ref}"
    --title "Automic Vault ${version}"
  )

  cli_require_tool gh
  cli_step "Creating draft GitHub release ${tag}"
  gh release create "${release_args[@]}" >&2

  cli_step "Uploading DMG to GitHub release"
  if ! gh release upload "${tag}" "${dmg_path}#${asset_label}" >&2; then
    cli_error "DMG upload failed"
    cli_die "Draft release remains unpublished: ${tag}"
  fi

  publish_public_dmg "${dmg_path}" "${tag}"

  cli_step "Publishing GitHub release ${tag}"
  if ! gh release edit "${tag}" --draft=false >&2; then
    cli_error "Release publish failed"
    cli_die "Draft release remains unpublished: ${tag}"
  fi
}

clobber_github_release() {
  local tag="$1"
  local notes_path
  local view_error

  cli_require_tool gh
  notes_path="$(mktemp "${TMPDIR:-/tmp}/automic-vault-release-notes.XXXXXX")"
  view_error="$(mktemp "${TMPDIR:-/tmp}/automic-vault-release-view.XXXXXX")"

  if ! gh release view "${tag}" --json body --jq '.body' >"${notes_path}" 2>"${view_error}"; then
    if grep -Eiq 'release not found|not found|HTTP 404' "${view_error}"; then
      rm -f "${view_error}"
      printf 'Rebuilt release %s.\n' "${tag}" >"${notes_path}"
      printf '%s\n' "${notes_path}"
      return 0
    fi

    cat "${view_error}" >&2
    rm -f "${notes_path}" "${view_error}"
    cli_die "Unable to check existing GitHub release ${tag}"
  fi

  rm -f "${view_error}"
  if [[ ! -s "${notes_path}" ]]; then
    printf 'Rebuilt release %s.\n' "${tag}" >"${notes_path}"
  fi

  cli_step "Clobbering existing GitHub release ${tag}"
  if ! gh release delete "${tag}" --yes --cleanup-tag >&2; then
    rm -f "${notes_path}"
    cli_die "Unable to clobber existing GitHub release ${tag}"
  fi

  printf '%s\n' "${notes_path}"
}

latest_release_tag_before() {
  local target_tag="$1"
  local release_tag
  local releases

  if ! releases="$(
    gh release list \
      --exclude-drafts \
      --limit 50 \
      --json tagName \
      --jq '.[].tagName'
  )"; then
    cli_die "Unable to list GitHub releases"
  fi

  while IFS= read -r release_tag; do
    if [[ -n "${release_tag}" && "${release_tag}" != "${target_tag}" ]]; then
      printf '%s\n' "${release_tag}"
      return 0
    fi
  done <<<"${releases}"

  return 1
}

ensure_git_tag_available() {
  local tag="$1"

  if git -C "${repo_root}" rev-parse --verify --quiet "${tag}^{commit}" >/dev/null; then
    return 0
  fi

  cli_step "Fetching release tag ${tag}"
  if ! git -C "${repo_root}" fetch --quiet origin "refs/tags/${tag}:refs/tags/${tag}"; then
    cli_die "Unable to fetch release tag ${tag}"
  fi
}

generate_release_notes() {
  local tag="$1"
  local target_ref="$2"
  local notes_path
  local previous_tag
  local prompt

  cli_require_tool codex
  cli_require_tool gh

  notes_path="$(mktemp "${TMPDIR:-/tmp}/automic-vault-release-notes.XXXXXX")"

  if previous_tag="$(latest_release_tag_before "${tag}")"; then
    ensure_git_tag_available "${previous_tag}"
    prompt="Summarize the user-facing changes in Automic Vault since the last release.

Repository: ${repo_root}
Previous release tag: ${previous_tag}
New release tag: ${tag}
Compare range: ${previous_tag}..${target_ref}

Inspect the git history and diff for that range. Write concise GitHub release notes in Markdown.
Focus on behavior, fixes, user-visible improvements, packaging, and operational changes.
Do not include a title, preamble, commit hashes, contributor lists, or references to GitHub auto-generated notes.
Do not edit files or create commits.
Use short bullets grouped under clear headings only when useful."
  else
    prompt="Write initial GitHub release notes for Automic Vault.

Repository: ${repo_root}
New release tag: ${tag}
Target ref: ${target_ref}

Inspect the repository and recent git history. Write concise GitHub release notes in Markdown.
Focus on behavior, fixes, user-visible improvements, packaging, and operational changes.
Do not include a title, preamble, commit hashes, contributor lists, or references to GitHub auto-generated notes.
Do not edit files or create commits.
Use short bullets grouped under clear headings only when useful."
  fi

  cli_step "Generating release notes with Codex"
  if ! codex exec \
    --cd "${repo_root}" \
    --sandbox read-only \
    --config approval_policy=\"never\" \
    --color never \
    --ephemeral \
    --output-last-message "${notes_path}" \
    "${prompt}" \
    >&2; then
    cli_die "Codex release note generation failed"
  fi

  if [[ ! -s "${notes_path}" ]]; then
    cli_die "Codex generated empty release notes"
  fi

  printf '%s\n' "${notes_path}"
}

publish_public_dmg() {
  local dmg_path="$1"
  local tag="$2"
  local distribution_id

  cli_require_tool aws
  cli_step "Uploading DMG to ${release_s3_uri}"
  if ! aws s3 cp \
    "${dmg_path}" \
    "${release_s3_uri}" \
    --content-type application/x-apple-diskimage \
    >&2; then
    cli_error "S3 upload failed"
    cli_die "Draft release remains unpublished: ${tag}"
  fi

  distribution_id="${AUTOMIC_VAULT_CLOUDFRONT_DISTRIBUTION_ID:-}"
  if [[ -z "${distribution_id}" ]]; then
    cli_step "Finding CloudFront distribution for ${release_cloudfront_alias}"
    if ! distribution_id="$(
        aws cloudfront list-distributions \
        --query "DistributionList.Items[?Aliases.Items && contains(join(',', Aliases.Items), '${release_cloudfront_alias}')].Id | [0]" \
        --output text
      )"; then
      cli_error "CloudFront distribution lookup failed"
      cli_die "Draft release remains unpublished: ${tag}"
    fi
  fi

  if [[ -z "${distribution_id}" || "${distribution_id}" == "None" ]]; then
    cli_die "Unable to find CloudFront distribution for ${release_cloudfront_alias}"
  fi

  cli_step "Invalidating CloudFront path ${release_cloudfront_path}"
  if ! aws cloudfront create-invalidation \
    --distribution-id "${distribution_id}" \
    --paths "${release_cloudfront_path}" \
    >&2; then
    cli_error "CloudFront invalidation failed"
    cli_die "Draft release remains unpublished: ${tag}"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output)
      output_path="$2"
      shift 2
      ;;
    --background)
      background_path="$2"
      shift 2
      ;;
    --volume-name)
      volume_name="$2"
      shift 2
      ;;
    --notorize|--notarize)
      notarize=true
      shift
      ;;
    --install)
      install_app=true
      shift
      ;;
    --publish)
      publish_release=true
      shift
      ;;
    --clobber)
      clobber_release=true
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      cli_error "Unknown argument: $1"
      usage >&2
      exit 1
      ;;
  esac
done

if [[ "${publish_release}" == "true" && "${notarize}" != "true" ]]; then
  cli_die "--publish requires --notarize"
fi

if [[ "${clobber_release}" == "true" && "${publish_release}" != "true" ]]; then
  cli_die "--clobber requires --publish"
fi

if [[ -z "${background_path}" && -f "${default_background}" ]]; then
  background_path="${default_background}"
fi

if [[ -n "${background_path}" && ! -f "${background_path}" ]]; then
  cli_die "Background image not found: ${background_path}"
fi

# if [[ -n "${background_path}" ]]; then
#   finder_width="$(
#     sips -g pixelWidth "${background_path}" 2>/dev/null |
#       awk '/pixelWidth:/ {print $2; exit}'
#   )"
#   finder_height="$(
#     sips -g pixelHeight "${background_path}" 2>/dev/null |
#       awk '/pixelHeight:/ {print $2; exit}'
#   )"
# fi

applications_icon_y=$(((finder_height / 2 - 60) * 6 / 5))
app_icon_y="${applications_icon_y}"

cli_title "Build Automic Vault DMG"
cli_step "Building release app bundle"
build_app_args=(--release)
if [[ "${publish_release}" == "true" ]]; then
  build_app_args+=(--publish)
fi
app_path="$("${repo_root}/scripts/build-app.sh" "${build_app_args[@]}")"
app_name="$(basename "${app_path}")"
app_stem="${app_name%.app}"
plist_path="${app_path}/Contents/Info.plist"
icon_name="$(
  /usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' "${plist_path}" \
    2>/dev/null || printf 'gui-icon'
)"
volume_icon_path="${app_path}/Contents/Resources/${icon_name}.icns"

version="$(
  /usr/libexec/PlistBuddy -c \
    'Print :CFBundleShortVersionString' \
    "${plist_path}" 2>/dev/null || printf '0.1'
)"

if [[ -z "${volume_name}" ]]; then
  volume_name="${app_stem}"
fi

safe_version="${version// /-}"
default_output="${target_dir}/${app_stem// /-}-${safe_version}.dmg"

if [[ -z "${output_path}" ]]; then
  output_path="${default_output}"
fi

mkdir -p "${build_dir}" "${target_dir}"
mkdir -p "$(dirname "${output_path}")"
output_dir="$(cd "$(dirname "${output_path}")" && pwd)"
output_path="${output_dir}/$(basename "${output_path}")"

final_dmg="${output_path}"
cli_info "Version: ${version}"
cli_info "Output: ${final_dmg}"
if [[ -n "${background_path}" ]]; then
  cli_info "Background: ${background_path}"
fi

rm -f "${final_dmg}"
create_dmg_args=(
  --volname "${volume_name}"
  --window-pos "${finder_left}" "${finder_top}"
  --window-size "${finder_width}" "${finder_height}"
  --icon-size "${icon_size}"
  --icon "${app_name}" "${app_icon_x}" "${app_icon_y}"
  --app-drop-link "${applications_icon_x}" "${applications_icon_y}"
  --format ULFO
  --filesystem APFS
  --hdiutil-quiet
)

if [[ -n "${background_path}" ]]; then
  create_dmg_args+=(
    --background "${background_path}"
  )
fi

if [[ -f "${volume_icon_path}" ]]; then
  create_dmg_args+=(
    --volicon "${volume_icon_path}"
  )
fi

cli_require_tool create-dmg
cli_step "Composing disk image"
create-dmg \
  "${create_dmg_args[@]}" \
  "${final_dmg}" \
  "${app_path}" \
  >&2

if [[ "${notarize}" == "true" ]]; then
  cli_step "Submitting DMG for notarization"
  if [[ -z "${APPLE_USERNAME:-}" ]]; then
    cli_die "APPLE_USERNAME is required for notarization"
  fi

  if [[ -z "${APPLE_PASSWORD:-}" ]]; then
    cli_die "APPLE_PASSWORD is required for notarization"
  fi

  if [[ -z "${CODESIGN_IDENTITY:-}" ]]; then
    cli_die "CODESIGN_IDENTITY is required for notarization"
  fi

  if [[ "${CODESIGN_IDENTITY}" =~ \(([A-Z0-9]+)\)[[:space:]]*$ ]]; then
    team_id="${BASH_REMATCH[1]}"
  else
    cli_error "Unable to extract Apple team ID from CODESIGN_IDENTITY"
    cli_die "Expected an identity like: Developer ID Application: Name (TEAMID)"
  fi

  xcrun notarytool submit \
    --apple-id "${APPLE_USERNAME}" \
    --team-id "${team_id}" \
    --password "${APPLE_PASSWORD}" \
    --wait \
    "${final_dmg}" \
    >&2

  cli_step "Stapling notarization ticket"
  xcrun stapler staple "${final_dmg}" >&2
fi

if [[ "${install_app}" == "true" ]]; then
  cli_step "Installing app into /Applications"
  install_mount="$(mktemp -d "${TMPDIR:-/tmp}/automic-vault-install.XXXXXX")"
  cleanup_install_mount() {
    hdiutil detach "${install_mount}" >/dev/null 2>&1 || true
    rmdir "${install_mount}" >/dev/null 2>&1 || true
  }
  trap cleanup_install_mount EXIT

  hdiutil attach \
    -nobrowse \
    -readonly \
    -mountpoint "${install_mount}" \
    "${final_dmg}" \
    >/dev/null

  mounted_app_path="${install_mount}/${app_name}"
  install_path="/Applications/${app_name}"
  rm -rf "${install_path}"
  ditto "${mounted_app_path}" "${install_path}"
  sudo cp -f "${mounted_app_path}/Contents/Resources/av" /usr/local/bin/av
  sudo chmod 755 /usr/local/bin/av
fi

if [[ "${publish_release}" == "true" ]]; then
  if [[ ! "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    cli_die "Release publishing requires an X.Y.Z version, got: ${version}"
  fi

  publish_github_release "v${version}" "${version}" "${final_dmg}"
fi

cli_done "DMG ready"
echo "${final_dmg}"
