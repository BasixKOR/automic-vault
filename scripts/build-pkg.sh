#!/usr/local/bin/av inject +APPLE_PASSWORD /bin/zsh

set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
source "${repo_root}/scripts/cli-style.sh"
cli_style_init "Automic Vault"
export COPYFILE_DISABLE=1
target_dir="${repo_root}/target"
work_dir="${target_dir}/pkg"
payload_root="${work_dir}/root"
scripts_dir="${work_dir}/scripts"
resources_dir="${work_dir}/resources"
component_pkg="${work_dir}/AutomicVaultPayload.pkg"
distribution_path="${work_dir}/Distribution.xml"
welcome_path="${resources_dir}/Welcome.txt"
upload_uri="s3://automicvault.com/av.pkg"
output_path=""
sign_pkg=true
notarize_pkg=true
upload_pkg=true
installer_identity="${INSTALLER_IDENTITY:-}"
notary_team_id=""
notary_team_id_source=""

usage() {
  cat <<'EOF'
Usage: scripts/build-pkg.sh [--output PATH] [--installer-identity NAME]
                            [--no-sign] [--no-notarize] [--no-upload]

Build Automic Vault's macOS installer package.

The package installs Automic Vault.app for the active console user, installs
the bundled av CLI at /usr/local/bin/av, installs and activates the privileged
root helper, then optionally notarizes and uploads the final package to
s3://automicvault.com/av.pkg.

Options:
  --output PATH              Write the final PKG to PATH.
  --installer-identity NAME  Developer ID Installer identity for productbuild.
  --no-sign                  Build an unsigned package for local testing.
  --no-notarize              Skip notarytool submission and stapling.
  --no-upload                Skip the S3 upload.
  --help                     Show this help.
EOF
}

load_build_env() {
  local env_file="${repo_root}/.env"
  [[ -f "$env_file" ]] || return

  local line key value
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%$'\r'}"
    [[ -n "$line" && "$line" != \#* && "$line" =~ '^[A-Za-z_][A-Za-z0-9_]*=' ]] || continue

    key="${line%%=*}"
    value="${line#*=}"
    if (( ! ${+parameters[$key]} )); then
      export "$key=$value"
    fi
  done <"$env_file"
}

infer_installer_identity() {
  security find-identity -v -p basic 2>/dev/null |
    sed -n 's/.*"\(Developer ID Installer:.*\)"/\1/p' |
    head -n 1
}

team_id_from_identity() {
  local identity="$1"
  [[ -n "$identity" ]] || return

  local team_id
  team_id="$(
    printf '%s\n' "$identity" |
      sed -n 's/.*(\([A-Z0-9][A-Z0-9]*\))[[:space:]]*$/\1/p'
  )"
  if [[ -n "$team_id" ]]; then
    printf '%s\n' "$team_id"
    return
  fi

  local line
  while IFS= read -r line; do
    if [[ "$line" == *"$identity"* ]]; then
      team_id="$(
        printf '%s\n' "$line" |
          sed -n 's/.*(\([A-Z0-9][A-Z0-9]*\))"[[:space:]]*$/\1/p'
      )"
      if [[ -n "$team_id" ]]; then
        printf '%s\n' "$team_id"
        return
      fi
    fi
  done < <(security find-identity -v -p basic 2>/dev/null)
}

resolve_notarization_team_id() {
  notary_team_id=""
  notary_team_id_source=""

  if [[ -n "${APPLE_TEAM_ID:-}" ]]; then
    notary_team_id="$APPLE_TEAM_ID"
    notary_team_id_source="APPLE_TEAM_ID"
    return
  fi

  notary_team_id="$(team_id_from_identity "${CODESIGN_IDENTITY:-}")"
  if [[ -n "$notary_team_id" ]]; then
    notary_team_id_source="CODESIGN_IDENTITY"
    return
  fi

  notary_team_id="$(team_id_from_identity "$installer_identity")"
  if [[ -n "$notary_team_id" ]]; then
    notary_team_id_source="INSTALLER_IDENTITY"
  fi
}

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    cli_die "${tool} is required"
  fi
}

require_developer_id_installer() {
  if [[ "$installer_identity" != Developer\ ID\ Installer:* ]]; then
    cli_error "INSTALLER_IDENTITY must be a Developer ID Installer identity"
    cli_info "Current identity: ${installer_identity}"
    cli_die "Use a Developer ID Installer identity for notarized packages."
  fi
}

submission_id_from_notary_output() {
  awk '/^[[:space:]]*id:/ { print $2; exit }' "$1"
}

print_notary_failure() {
  local submit_output="$1"
  local submission_id="$2"
  shift 2

  cat "$submit_output" >&2
  if [[ -z "$submission_id" ]]; then
    cli_error "Unable to read notary submission ID."
    return
  fi

  cli_info "Fetching notary log for ${submission_id}"
  xcrun notarytool log "$submission_id" "$@" >&2 || true
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output)
      output_path="$2"
      shift 2
      ;;
    --installer-identity)
      installer_identity="$2"
      shift 2
      ;;
    --no-sign)
      sign_pkg=false
      notarize_pkg=false
      shift
      ;;
    --no-notarize|--no-notorize)
      notarize_pkg=false
      shift
      ;;
    --no-upload)
      upload_pkg=false
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

load_build_env
require_tool pkgbuild
require_tool productbuild

if [[ "$sign_pkg" == "true" && -z "$installer_identity" ]]; then
  installer_identity="$(infer_installer_identity)"
fi

if [[ "$sign_pkg" == "true" && -z "$installer_identity" ]]; then
  cli_error "INSTALLER_IDENTITY or --installer-identity is required"
  cli_die "Expected a Developer ID Installer identity."
fi

if [[ "$notarize_pkg" == "true" ]]; then
  require_developer_id_installer
fi

if [[ "$notarize_pkg" == "true" && -z "${CODESIGN_IDENTITY:-}" ]]; then
  cli_die "CODESIGN_IDENTITY is required for notarized release packages"
fi

if [[ "$notarize_pkg" == "true" && -z "${NOTARYTOOL_PROFILE:-}" ]]; then
  resolve_notarization_team_id
  if [[ -n "$notary_team_id" ]]; then
    cli_info "Notarization team ID source: ${notary_team_id_source}"
    cli_info "Notarization team ID: ${notary_team_id}"
  else
    cli_warn "Notarization team ID source: none"
    cli_die "APPLE_TEAM_ID is required for notarization"
  fi
fi

cli_title "Build Automic Vault PKG"
cli_step "Building release app bundle"
app_path="$("${repo_root}/scripts/build-app.sh" --release)"
app_name="$(basename "$app_path")"
plist_path="${app_path}/Contents/Info.plist"
version="$(
  /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' \
    "$plist_path" 2>/dev/null || printf '0.1.0'
)"
safe_version="${version// /-}"

if [[ -z "$output_path" ]]; then
  output_path="${target_dir}/Automic-Vault-${safe_version}.pkg"
fi

mkdir -p "$target_dir" "$(dirname "$output_path")"
output_dir="$(cd "$(dirname "$output_path")" && pwd)"
output_path="${output_dir}/$(basename "$output_path")"
cli_info "Version: ${version}"
cli_info "Output: ${output_path}"
if [[ "$sign_pkg" == "true" ]]; then
  cli_info "Installer identity: ${installer_identity}"
else
  cli_warn "Package signing disabled"
fi

rm -rf "$work_dir"
mkdir -p \
  "${payload_root}/Library/Application Support/Automic Vault/InstallerPayload" \
  "$scripts_dir" \
  "$resources_dir"

payload_dir="${payload_root}/Library/Application Support/Automic Vault/InstallerPayload"
cli_step "Staging installer payload"
ditto --norsrc "$app_path" "${payload_dir}/${app_name}"
cp "${app_path}/Contents/Resources/av" "${payload_dir}/av"
cp "${app_path}/Contents/Library/LaunchServices/com.automicvault.nuke-helper" \
  "${payload_dir}/com.automicvault.nuke-helper"
cp "${repo_root}/src/helper/launchd.plist" \
  "${payload_dir}/com.automicvault.nuke-helper.plist"
chmod 755 \
  "${payload_dir}/av" \
  "${payload_dir}/com.automicvault.nuke-helper"
chmod 644 "${payload_dir}/com.automicvault.nuke-helper.plist"

cat >"${scripts_dir}/postinstall" <<'POSTINSTALL'
#!/bin/zsh
set -euo pipefail

payload_dir="/Library/Application Support/Automic Vault/InstallerPayload"
app_name="Automic Vault.app"
service_name="com.automicvault.nuke-helper"
app_source="${payload_dir}/${app_name}"
av_source="${payload_dir}/av"
helper_source="${payload_dir}/${service_name}"
launchd_source="${payload_dir}/${service_name}.plist"
helper_target="/Library/PrivilegedHelperTools/${service_name}"
launchd_target="/Library/LaunchDaemons/${service_name}.plist"

console_user="$(stat -f %Su /dev/console)"
if [[ -z "$console_user" || "$console_user" == "root" ||
      "$console_user" == "loginwindow" ]]; then
  echo "No active console user is available for the app install." >&2
  exit 1
fi

console_home="$(dscl . -read "/Users/${console_user}" NFSHomeDirectory |
  awk '{print $2; exit}')"
if [[ -z "$console_home" || ! -d "$console_home" ]]; then
  echo "Unable to resolve home directory for ${console_user}." >&2
  exit 1
fi

console_uid="$(id -u "$console_user")"
console_gid="$(id -g "$console_user")"
user_apps="${console_home}/Applications"
app_target="${user_apps}/${app_name}"

mkdir -p "$user_apps" /usr/local/bin /Library/PrivilegedHelperTools
ditto --norsrc "$app_source" "$app_target"
chown "${console_uid}:${console_gid}" "$user_apps"
chown -R "${console_uid}:${console_gid}" "$app_target"

install -o root -g wheel -m 755 "$av_source" /usr/local/bin/av
install -o root -g wheel -m 755 "$helper_source" "$helper_target"
install -o root -g wheel -m 644 "$launchd_source" "$launchd_target"

launchctl bootout system "$launchd_target" >/dev/null 2>&1 || true
launchctl bootstrap system "$launchd_target"
launchctl enable "system/${service_name}" >/dev/null 2>&1 || true
launchctl kickstart -k "system/${service_name}" >/dev/null 2>&1 || true

rm -rf "$payload_dir"
POSTINSTALL
chmod 755 "${scripts_dir}/postinstall"

xattr -cr "$payload_root" "$scripts_dir" "$resources_dir" 2>/dev/null || true

cli_step "Writing installer resources"
cat >"$welcome_path" <<'WELCOME'
Automic Vault will install the following components:

- Automic Vault.app into the active user's ~/Applications folder.
- The av command line tool at /usr/local/bin/av.
- The signed privileged helper at /Library/PrivilegedHelperTools.

The installer activates the privileged helper with launchd so Automic Vault can
perform root-owned package operations after installation. macOS may request an
administrator password before any files are installed.
WELCOME

cat >"$distribution_path" <<DISTRIBUTION
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
  <title>Automic Vault</title>
  <welcome file="Welcome.txt" mime-type="text/plain"/>
  <options customize="never" require-scripts="true"/>
  <domains enable_anywhere="false" enable_currentUserHome="false"
      enable_localSystem="true"/>
  <choices-outline>
    <line choice="default">
      <line choice="com.automicvault.pkg.payload"/>
    </line>
  </choices-outline>
  <choice id="default"/>
  <choice id="com.automicvault.pkg.payload"
      title="Automic Vault ${version}">
    <pkg-ref id="com.automicvault.pkg.payload"/>
  </choice>
  <pkg-ref id="com.automicvault.pkg.payload"
      version="${version}"
      onConclusion="none">AutomicVaultPayload.pkg</pkg-ref>
</installer-gui-script>
DISTRIBUTION

rm -f "$component_pkg" "$output_path"
cli_step "Building component package"
pkgbuild \
  --root "$payload_root" \
  --scripts "$scripts_dir" \
  --identifier com.automicvault.pkg.payload \
  --version "$version" \
  --install-location / \
  "$component_pkg" \
  >&2

productbuild_args=(
  --distribution "$distribution_path"
  --package-path "$work_dir"
  --resources "$resources_dir"
)

if [[ "$sign_pkg" == "true" ]]; then
  productbuild_args+=(--sign "$installer_identity")
fi

cli_step "Building product package"
productbuild "${productbuild_args[@]}" "$output_path" >&2

if [[ "$notarize_pkg" == "true" ]]; then
  cli_step "Submitting package for notarization"
  notary_output="$(mktemp "${TMPDIR:-/tmp}/automic-vault-notary.XXXXXX")"
  notary_status=0
  submission_id=""

  if [[ -n "${NOTARYTOOL_PROFILE:-}" ]]; then
    set +e
    xcrun notarytool submit \
      --keychain-profile "$NOTARYTOOL_PROFILE" \
      --wait \
      "$output_path" 2>&1 | tee "$notary_output" >&2
    notary_status="${pipestatus[1]}"
    set -e
    submission_id="$(submission_id_from_notary_output "$notary_output")"
    if [[ "$notary_status" -ne 0 ]] ||
        ! grep -q '^[[:space:]]*status: Accepted[[:space:]]*$' "$notary_output"; then
      print_notary_failure \
        "$notary_output" \
        "$submission_id" \
        --keychain-profile "$NOTARYTOOL_PROFILE"
      exit 1
    fi
  else
    if [[ -z "${APPLE_USERNAME:-}" ]]; then
      cli_die "APPLE_USERNAME is required for notarization"
    fi
    if [[ -z "${APPLE_PASSWORD:-}" ]]; then
      cli_die "APPLE_PASSWORD is required for notarization"
    fi
    if [[ -z "$notary_team_id" ]]; then
      cli_die "APPLE_TEAM_ID is required for notarization"
    fi
    set +e
    xcrun notarytool submit \
      --apple-id "$APPLE_USERNAME" \
      --team-id "$notary_team_id" \
      --password "$APPLE_PASSWORD" \
      --wait \
      "$output_path" 2>&1 | tee "$notary_output" >&2
    notary_status="${pipestatus[1]}"
    set -e
    submission_id="$(submission_id_from_notary_output "$notary_output")"
    if [[ "$notary_status" -ne 0 ]] ||
        ! grep -q '^[[:space:]]*status: Accepted[[:space:]]*$' "$notary_output"; then
      print_notary_failure \
        "$notary_output" \
        "$submission_id" \
        --apple-id "$APPLE_USERNAME" \
        --team-id "$notary_team_id" \
        --password "$APPLE_PASSWORD"
      exit 1
    fi
  fi

  cli_step "Stapling notarization ticket"
  xcrun stapler staple "$output_path" >&2
fi

if [[ "$upload_pkg" == "true" ]]; then
  require_tool aws
  cli_step "Uploading package to ${upload_uri}"
  aws s3 cp \
    "$output_path" \
    "$upload_uri" \
    --content-type application/octet-stream \
    >&2
fi

cli_done "PKG ready"
echo "$output_path"
