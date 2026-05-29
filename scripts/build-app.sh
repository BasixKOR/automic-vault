#!/bin/zsh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
GUI_DIR="$ROOT_DIR/src/gui"
CONFIGURATION="${GUI_BUILD_CONFIGURATION:-debug}"
PUBLISH_BUILD=false
source "$ROOT_DIR/scripts/cli-style.sh"
cli_style_init "Automic Vault"

load_build_env() {
  local env_file="$ROOT_DIR/.env"
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

rust_protocol_version() {
  awk -F'"' '/PROTOCOL_VERSION[[:space:]]*:/ { print $2; exit }' "$ROOT_DIR/src/lib/rs/core.rs"
}

usage() {
  cat <<'EOF'
Usage: scripts/build-app.sh [--debug|--release] [--publish]

Build Automic Vault.app and print the app bundle path.

Options:
  --debug       Build faster local debug binaries. This is the default.
  --release     Build optimized release binaries for packaging.
  --publish     Require a current package database. Use for published builds.
  --help        Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --debug)
      CONFIGURATION="debug"
      shift
      ;;
    --release)
      CONFIGURATION="release"
      shift
      ;;
    --publish)
      PUBLISH_BUILD=true
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

case "$CONFIGURATION" in
  debug|release)
    ;;
  *)
    cli_error "Unknown GUI_BUILD_CONFIGURATION: $CONFIGURATION"
    usage >&2
    exit 1
    ;;
esac

BUILD_DIR="$ROOT_DIR/target/gui/$CONFIGURATION"
APP_DIR="$BUILD_DIR/Automic Vault.app"
MACOS_DIR="$APP_DIR/Contents/MacOS"
RESOURCES_DIR="$APP_DIR/Contents/Resources"
HELPERS_DIR="$APP_DIR/Contents/Library/LaunchServices"
LOGIN_ITEMS_DIR="$APP_DIR/Contents/Library/LoginItems"
EXECUTABLE="$MACOS_DIR/Automic Vault"
HELPER_EXECUTABLE="$HELPERS_DIR/com.automicvault.nuke-helper"
MENU_APP_DIR="$LOGIN_ITEMS_DIR/Automic Vault Menu.app"
MENU_MACOS_DIR="$MENU_APP_DIR/Contents/MacOS"
MENU_RESOURCES_DIR="$MENU_APP_DIR/Contents/Resources"
MENU_EXECUTABLE="$MENU_MACOS_DIR/Automic Vault Menu"
ICON_PNG="$ROOT_DIR/assets/gui-icon.png"
MENU_APP_ICON_NAME="gui-icon"
MENU_ICON_PNG="$ROOT_DIR/assets/NSMenuItem.png"
MENU_ICON_1X="$BUILD_DIR/NSMenuItem.png"
MENU_ICON_2X="$BUILD_DIR/NSMenuItem@2x.png"
ENRICHMENT_MANIFESTS_JSON="$BUILD_DIR/enrichment-manifests.json"
ICON_NAME="gui-icon"
ICONSET_DIR="$BUILD_DIR/$ICON_NAME.iconset"
ICON_ICNS="$BUILD_DIR/$ICON_NAME.icns"
[[ -n "${MIN_MACOS_VERSION:-}" ]] || cli_die "Set MIN_MACOS_VERSION in .env"
NUKE_PROTOCOL_VERSION="$(rust_protocol_version)"
[[ -n "$NUKE_PROTOCOL_VERSION" ]] || cli_die "Could not read PROTOCOL_VERSION from src/lib/rs/core.rs"
[[ -n "${NUKE_HELPER_VERSION:-}" ]] || cli_die "Set NUKE_HELPER_VERSION in .env"
APP_VERSION="$(awk -F'\"' '/^version = / { print $2; exit }' "$ROOT_DIR/Cargo.toml")"
APPLE_TEAM_ID="${APPLE_TEAM_ID:-}"

git_build_id() {
  git -C "$ROOT_DIR" rev-parse --short=12 HEAD 2>/dev/null || printf '%s' "$APP_VERSION"
}

if [[ "$CONFIGURATION" == "release" || "$PUBLISH_BUILD" == "true" ]]; then
  # Production builds intentionally let build.rs compute and track the Git ID.
  APP_BUILD_ID="$(git_build_id)"
  unset NUKE_BUILD_ID
elif [[ -n "${NUKE_BUILD_ID:-}" ]]; then
  APP_BUILD_ID="$NUKE_BUILD_ID"
  export NUKE_BUILD_ID
else
  # Local target/gui apps force a fresh daemon at launch, so a stable ID avoids
  # recompiling Rust for Swift-only commits while keeping app and daemon aligned.
  APP_BUILD_ID="local-${APP_VERSION}"
  export NUKE_BUILD_ID="$APP_BUILD_ID"
fi

APP_BUNDLE_ID="com.automicvault"
MENU_BUNDLE_ID="com.automicvault.menu-helper"
HELPER_BUNDLE_ID="com.automicvault.nuke-helper"

cli_step "Validating package database"
if package_database_check="$("$ROOT_DIR/scripts/build-combined-json.py" --check 2>&1)"; then
  cli_info "${package_database_check}"
elif [[ "$PUBLISH_BUILD" == "true" ]]; then
  cli_error "${package_database_check}"
  cli_die "Package database must be current for published builds."
else
  cli_warn "${package_database_check}"
  cli_step "Regenerating package database"
  "$ROOT_DIR/scripts/build-combined-json.py"
  package_database_check="$("$ROOT_DIR/scripts/build-combined-json.py" --check 2>&1)"
  cli_info "${package_database_check}"
fi

if [[ -z "$APPLE_TEAM_ID" && -n "${CODESIGN_IDENTITY:-}" ]]; then
  if [[ "${CODESIGN_IDENTITY}" =~ \(([A-Z0-9]+)\)[[:space:]]*$ ]]; then
    APPLE_TEAM_ID="${match[1]}"
  fi
fi

if [[ -n "$APPLE_TEAM_ID" ]]; then
  HELPER_REQUIREMENT="identifier \"$HELPER_BUNDLE_ID\" and anchor apple generic and certificate leaf[subject.OU] = \"$APPLE_TEAM_ID\""
else
  HELPER_REQUIREMENT="identifier \"$HELPER_BUNDLE_ID\" and anchor apple generic"
fi

if [[ "$CONFIGURATION" == "release" ]]; then
  [[ -n "${POSTHOG_API_KEY:-}" ]] || cli_die "Set POSTHOG_API_KEY in the environment for release GUI builds"
fi

if [[ -n "$APPLE_TEAM_ID" ]]; then
  export APPLE_TEAM_ID
else
  unset APPLE_TEAM_ID
fi
export NUKE_HELPER_VERSION
SHARED_SWIFT_SOURCES=(
  "$GUI_DIR/PackageModels.swift"
  "$GUI_DIR/SecurityCatalog.swift"
  "$GUI_DIR/NucleusBridge.swift"
  "$GUI_DIR/NukeHelperBridge.swift"
  "$GUI_DIR/NucleusStatusStore.swift"
  "$GUI_DIR/VaultApprovalStore.swift"
  "$GUI_DIR/ContainmentLogStore.swift"
)
GUI_SWIFT_SOURCES=(
  "$GUI_DIR/AppMain.swift"
  "$GUI_DIR/AppDelegate.swift"
  "$GUI_DIR/PackageNodeHazardEffect.swift"
  "$GUI_DIR/RootViewController.swift"
  "$GUI_DIR/PackageFieldView.swift"
  "$GUI_DIR/DossierView.swift"
  "$GUI_DIR/ExternalSurfaceView.swift"
  "$GUI_DIR/UpdateProgressViewController.swift"
  "$GUI_DIR/ContainmentLogWindowController.swift"
  "$GUI_DIR/UIStyle.swift"
)
MENU_SWIFT_SOURCES=(
  "$GUI_DIR/MenuBarMain.swift"
  "$GUI_DIR/MenuBarAppDelegate.swift"
  "$GUI_DIR/VaultDaemon.swift"
)

if [[ "$CONFIGURATION" == "release" ]]; then
  SWIFT_OPT_FLAGS=(-O)
else
  SWIFT_OPT_FLAGS=(-Onone -g -D DEBUG)
fi

RUST_BIN_DIR="$ROOT_DIR/target/release"
SWIFT_PACKAGE_BIN_DIR=""

is_current() {
  local output_path="$1"
  shift

  if [[ ! -e "$output_path" ]]; then
    return 1
  fi

  local input_path
  for input_path in "$@"; do
    if [[ -e "$input_path" && "$input_path" -nt "$output_path" ]]; then
      return 1
    fi
  done

  return 0
}

sign_binary() {
  local target_path="$1"
  local identifier="$2"
  local entitlements="${3:-}"

  local -a args=(
    --force
    --options runtime
    --sign "$CODESIGN_IDENTITY"
  )

  if [[ -n "$identifier" ]]; then
    args+=(
      --identifier "$identifier"
    )
  fi

  if [[ -n "$entitlements" ]]; then
    args+=(
      --entitlements "$entitlements"
    )
  fi

  codesign "${args[@]}" "$target_path"
}

sign_bundle() {
  local target_path="$1"
  local identifier="$2"
  local entitlements="${3:-}"

  local -a args=(
    --force
    --options runtime
    --sign "$CODESIGN_IDENTITY"
  )

  if [[ -n "$identifier" ]]; then
    args+=(
      --identifier "$identifier"
    )
  fi

  if [[ -n "$entitlements" ]]; then
    args+=(
      --entitlements "$entitlements"
    )
  fi

  codesign "${args[@]}" "$target_path"
}

adhoc_sign_binary() {
  local target_path="$1"
  local entitlements="${2:-}"

  local -a args=(
    --force
    --options runtime
    --sign -
  )

  if [[ -n "$entitlements" ]]; then
    args+=(
      --entitlements "$entitlements"
    )
  fi

  codesign "${args[@]}" "$target_path"
}

adhoc_sign_bundle() {
  local target_path="$1"
  local entitlements="${2:-}"

  local -a args=(
    --force
    --options runtime
    --sign -
  )

  if [[ -n "$entitlements" ]]; then
    args+=(
      --entitlements "$entitlements"
    )
  fi

  codesign "${args[@]}" "$target_path"
}

build_icon() {
  local source_png="$1"
  local iconset_dir="$2"
  local output_icns="$3"

  if is_current "$output_icns" "$source_png"; then
    return
  fi

  rm -rf "$iconset_dir" "$output_icns"
  mkdir -p "$iconset_dir"

  local -a icon_sizes=(16 32 128 256 512)
  local size
  for size in "${icon_sizes[@]}"; do
    sips -z "$size" "$size" "$source_png" \
      --out "$iconset_dir/icon_${size}x${size}.png" \
      >/dev/null

    local retina_size=$((size * 2))
    sips -z "$retina_size" "$retina_size" "$source_png" \
      --out "$iconset_dir/icon_${size}x${size}@2x.png" \
      >/dev/null
  done

  iconutil -c icns "$iconset_dir" -o "$output_icns"
}

generate_enrichment_manifest_index() {
  local output_path="$1"
  local manifest_dir="$ROOT_DIR/manifests/enrichments"
  local temp_path="${output_path}.tmp"
  local -a manifest_names=()
  local manifest_path

  if [[ -d "$manifest_dir" ]]; then
    for manifest_path in "$manifest_dir"/*.rs(N); do
      manifest_names+=("${${manifest_path:t}:r}")
    done
  fi

  {
    printf '[\n'
    local count="${#manifest_names[@]}"
    local index
    for (( index = 1; index <= count; index++ )); do
      local suffix=','
      if [[ "$index" -eq "$count" ]]; then
        suffix=''
      fi
      printf '  "%s"%s\n' "${manifest_names[$index]}" "$suffix"
    done
    printf ']\n'
  } >"$temp_path"

  if [[ -f "$output_path" ]] && cmp -s "$temp_path" "$output_path"; then
    rm -f "$temp_path"
  else
    mv "$temp_path" "$output_path"
  fi
}

cli_title "Build Automic Vault.app"
cli_info "Configuration: $CONFIGURATION"
cli_info "Output: $APP_DIR"

mkdir -p "$BUILD_DIR"
cli_step "Building Rust binaries"
cargo build \
  --release \
  --features packaged-db \
  --bin av \
  --bin nuke-helper \
  --manifest-path "$ROOT_DIR/Cargo.toml"
cli_step "Building Cocoa app"
xcrun swift build \
  --package-path "$GUI_DIR" \
  --configuration "$CONFIGURATION" \
  --product AutomicVaultApp \
  >&2
SWIFT_PACKAGE_BIN_DIR="$(
  xcrun swift build \
    --package-path "$GUI_DIR" \
    --configuration "$CONFIGURATION" \
    --show-bin-path |
    tail -n 1
)"
cli_step "Preparing icons and manifests"
build_icon "$ICON_PNG" "$ICONSET_DIR" "$ICON_ICNS"
if ! is_current "$MENU_ICON_1X" "$MENU_ICON_PNG"; then
  sips -z 27 27 "$MENU_ICON_PNG" --out "$MENU_ICON_1X" >/dev/null
fi
if ! is_current "$MENU_ICON_2X" "$MENU_ICON_PNG"; then
  cp "$MENU_ICON_PNG" "$MENU_ICON_2X"
fi
generate_enrichment_manifest_index "$ENRICHMENT_MANIFESTS_JSON"

if [[ "$CONFIGURATION" == "release" ]]; then
  rm -rf "$APP_DIR"
fi
mkdir -p \
  "$MACOS_DIR" \
  "$RESOURCES_DIR" \
  "$HELPERS_DIR" \
  "$MENU_MACOS_DIR" \
  "$MENU_RESOURCES_DIR"

cp "$SWIFT_PACKAGE_BIN_DIR/AutomicVaultApp" "$EXECUTABLE"

if ! is_current "$MENU_EXECUTABLE" "${SHARED_SWIFT_SOURCES[@]}" "${MENU_SWIFT_SOURCES[@]}"; then
  cli_step "Building menu bar helper"
  xcrun swiftc \
    "${SWIFT_OPT_FLAGS[@]}" \
    -target "$(uname -m)-apple-macos${MIN_MACOS_VERSION}" \
    -framework AppKit \
    -framework Foundation \
    -framework QuartzCore \
    -framework ServiceManagement \
    -framework UserNotifications \
    -o "$MENU_EXECUTABLE" \
    "${SHARED_SWIFT_SOURCES[@]}" \
    "${MENU_SWIFT_SOURCES[@]}"
fi

cli_step "Assembling app bundle"
cp "$RUST_BIN_DIR/av" "$RESOURCES_DIR/av"
cp "$ROOT_DIR/data/combined.json" "$RESOURCES_DIR/combined.json"
rm -f "$RESOURCES_DIR/isotopes.json"
cp "$ENRICHMENT_MANIFESTS_JSON" "$RESOURCES_DIR/enrichment-manifests.json"
cp "$RUST_BIN_DIR/nuke-helper" "$HELPER_EXECUTABLE"
cp "$ICON_ICNS" "$RESOURCES_DIR/$ICON_NAME.icns"
cp "$ICON_ICNS" "$MENU_RESOURCES_DIR/$MENU_APP_ICON_NAME.icns"
cp "$RUST_BIN_DIR/av" "$MENU_RESOURCES_DIR/av"
cp "$ROOT_DIR/data/combined.json" "$MENU_RESOURCES_DIR/combined.json"
rm -f "$MENU_RESOURCES_DIR/isotopes.json"
cp "$ENRICHMENT_MANIFESTS_JSON" "$MENU_RESOURCES_DIR/enrichment-manifests.json"
cp "$MENU_ICON_1X" "$MENU_RESOURCES_DIR/NSMenuItem.png"
cp "$MENU_ICON_2X" "$MENU_RESOURCES_DIR/NSMenuItem@2x.png"
chmod 755 \
  "$EXECUTABLE" \
  "$RESOURCES_DIR/av" \
  "$HELPER_EXECUTABLE" \
  "$MENU_EXECUTABLE" \
  "$MENU_RESOURCES_DIR/av"

cat >"$APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
"http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>Automic Vault</string>
  <key>CFBundleIconFile</key>
  <string>gui-icon</string>
  <key>CFBundleIdentifier</key>
  <string>${APP_BUNDLE_ID}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>Automic Vault</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>SMPrivilegedExecutables</key>
  <dict>
    <key>${HELPER_BUNDLE_ID}</key>
    <string>${HELPER_REQUIREMENT}</string>
  </dict>
  <key>CFBundleShortVersionString</key>
  <string>${APP_VERSION}</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>NukeBuildID</key>
  <string>${APP_BUILD_ID}</string>
  <key>NukeProtocolVersion</key>
  <string>${NUKE_PROTOCOL_VERSION}</string>
  <key>NukeHelperVersion</key>
  <string>${NUKE_HELPER_VERSION}</string>
  <key>LSMinimumSystemVersion</key>
  <string>${MIN_MACOS_VERSION}</string>
  <key>NSAppTransportSecurity</key>
  <dict>
    <key>NSAllowsArbitraryLoadsInWebContent</key>
    <true/>
  </dict>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST

if [[ "$CONFIGURATION" == "release" ]]; then
  /usr/bin/plutil \
    -insert PostHogAPIKey \
    -string "$POSTHOG_API_KEY" \
    "$APP_DIR/Contents/Info.plist"
fi

cat >"$MENU_APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
"http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>Automic Vault Menu</string>
  <key>CFBundleIconFile</key>
  <string>${MENU_APP_ICON_NAME}</string>
  <key>CFBundleIdentifier</key>
  <string>${MENU_BUNDLE_ID}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>Automic Vault Menu</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${APP_VERSION}</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSMinimumSystemVersion</key>
  <string>${MIN_MACOS_VERSION}</string>
  <key>LSUIElement</key>
  <true/>
  <key>NukeBuildID</key>
  <string>${APP_BUILD_ID}</string>
  <key>NukeProtocolVersion</key>
  <string>${NUKE_PROTOCOL_VERSION}</string>
</dict>
</plist>
PLIST

if [[ -n "${CODESIGN_IDENTITY:-}" ]]; then
  cli_step "Signing bundle with Developer ID"
  sign_binary "$RESOURCES_DIR/av" "${APP_BUNDLE_ID}.av"
  sign_binary "$MENU_RESOURCES_DIR/av" "${MENU_BUNDLE_ID}.av"
  sign_binary \
    "$HELPER_EXECUTABLE" \
    "$HELPER_BUNDLE_ID" \
    "$ROOT_DIR/src/helper/NukeHelper.entitlements"
  sign_binary "$MENU_EXECUTABLE" "$MENU_BUNDLE_ID"
  sign_bundle "$MENU_APP_DIR" "$MENU_BUNDLE_ID"
  sign_bundle \
    "$APP_DIR" \
    "$APP_BUNDLE_ID" \
    "$ROOT_DIR/src/gui/AutomicVault.entitlements"
else
  cli_step "Signing bundle ad-hoc"
  adhoc_sign_binary "$RESOURCES_DIR/av"
  adhoc_sign_binary "$MENU_RESOURCES_DIR/av"
  adhoc_sign_binary \
    "$HELPER_EXECUTABLE" \
    "$ROOT_DIR/src/helper/NukeHelper.entitlements"
  adhoc_sign_binary "$MENU_EXECUTABLE"
  adhoc_sign_bundle "$MENU_APP_DIR"
  adhoc_sign_bundle \
    "$APP_DIR" \
    "$ROOT_DIR/src/gui/AutomicVault.entitlements"
fi

cli_done "App bundle ready"
echo "$APP_DIR"
