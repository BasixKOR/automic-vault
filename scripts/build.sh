#!/usr/bin/env bash
set -euo pipefail

run=0
install=0
dmg=0
notarize=0
publish=0
clobber=0
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(
  awk -F '"' '
    /^\[package\]/ { package = 1; next }
    /^\[/ { package = 0 }
    package && /^[[:space:]]*version[[:space:]]*=/ { print $2; exit }
  ' "$ROOT/Cargo.toml"
)"
APP_VERSION="${APP_VERSION:-$VERSION}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --run) run=1 ;;
    --install) install=1 ;;
    --dmg) dmg=1 ;;
    --notarize) notarize=1 ;;
    --publish) publish=1; dmg=1; notarize=1 ;;
    --clobber) clobber=1 ;;
    *)
      echo "usage: $0 [--run] [--install] [--dmg] [--notarize] [--publish] [--clobber]" >&2
      exit 64
      ;;
  esac
  shift
done
if [[ "$notarize" -eq 1 && "$dmg" -ne 1 ]]; then
  echo "error: --notarize requires --dmg" >&2
  exit 64
fi
if [[ "$clobber" -eq 1 && "$publish" -ne 1 ]]; then
  echo "error: --clobber requires --publish" >&2
  exit 64
fi
if [[ "$publish" -eq 1 && -z "${POSTHOG_API_KEY:-}" ]]; then
  echo "error: --publish requires POSTHOG_API_KEY" >&2
  exit 64
fi
if [[ "$publish" -eq 1 && -z "$VERSION" ]]; then
  echo "error: could not read package.version from Cargo.toml" >&2
  exit 64
fi
if [[ "$publish" -eq 1 ]] && ! command -v gh >/dev/null 2>&1; then
  echo "error: --publish requires gh" >&2
  exit 64
fi
publish_release() {
  local tag="$1"
  local dmg="$2"
  local branch head
  head="$(git -C "$ROOT" rev-parse HEAD)"
  branch="$(git -C "$ROOT" branch --show-current)"
  if [[ -z "$branch" ]]; then
    echo "error: --publish requires a branch checkout" >&2
    exit 64
  fi
  git -C "$ROOT" push origin "HEAD:$branch"
  if [[ "$clobber" -eq 1 ]]; then
    git -C "$ROOT" tag -f "$tag" "$head"
    git -C "$ROOT" push --force origin "refs/tags/$tag"
    if gh release view "$tag" >/dev/null 2>&1; then
      gh release upload "$tag" "$dmg" --clobber
      return
    fi
  fi
  gh release create "$tag" "$dmg" \
    --target "$head" \
    --title "$tag" \
    --generate-notes
}

MENU_HELPER="$ROOT/src/menu-helper"
SWIFT_TARGET="$ROOT/target/swift"
APP="$SWIFT_TARGET/Automic Vault.app"
DMG="$SWIFT_TARGET/automic-vault-$VERSION.dmg"
DMG_STAGE="$SWIFT_TARGET/dmg"
DMG_MOUNT="$SWIFT_TARGET/dmg-mount"
ICON_BUILD="$SWIFT_TARGET/icon"
MENU_HELPER_PROFILE="$HOME/Library/MobileDevice/Provisioning Profiles/Automic_Vault_Developer_ID.provisionprofile"
MENU_HELPER_ENTITLEMENTS="$SWIFT_TARGET/menu-helper.entitlements.plist"
INSTALLED_APP="/Applications/Automic Vault.app"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"
LAUNCH_AGENTS="$CONTENTS/Library/LaunchAgents"
LAUNCH_AGENT_NAME="com.automicvault.menubar-helper"
LAUNCH_AGENT_PLIST="$LAUNCH_AGENTS/$LAUNCH_AGENT_NAME.plist"
INSTALLED_LAUNCH_AGENT="$HOME/Library/LaunchAgents/$LAUNCH_AGENT_NAME.plist"

cargo build --release --manifest-path "$ROOT/Cargo.toml"
swift build -c release --package-path "$MENU_HELPER" --build-path "$SWIFT_TARGET"
SWIFT_BIN="$(swift build -c release --package-path "$MENU_HELPER" --build-path "$SWIFT_TARGET" --show-bin-path)"

rm -rf "$APP" "$ICON_BUILD"
mkdir -p "$MACOS" "$RESOURCES" "$LAUNCH_AGENTS" "$ICON_BUILD"
cp "$SWIFT_BIN/AutomicVaultMenubar" "$MACOS/AutomicVaultMenubar"
cp "$MENU_HELPER/Info.plist" "$CONTENTS/Info.plist"
plutil -replace CFBundleShortVersionString -string "$APP_VERSION" "$CONTENTS/Info.plist"
plutil -replace CFBundleVersion -string "$APP_VERSION" "$CONTENTS/Info.plist"
if [[ "$publish" -eq 1 ]]; then
  plutil -insert PostHogAPIKey -string "$POSTHOG_API_KEY" "$CONTENTS/Info.plist"
fi
cp "$MENU_HELPER/LaunchAgent.plist" "$LAUNCH_AGENT_PLIST"
if [[ "$run" -eq 1 && "$install" -eq 0 ]]; then
  plutil -replace ProgramArguments -json "[\"$MACOS/AutomicVaultMenubar\"]" "$LAUNCH_AGENT_PLIST"
fi
cp "$MENU_HELPER/Resources/NSMenuItem.png" "$RESOURCES/NSMenuItem.png"
xcrun actool "$MENU_HELPER/Resources/AppIcon.icon" \
  --compile "$ICON_BUILD" \
  --platform macosx \
  --target-device mac \
  --minimum-deployment-target 26.0 \
  --app-icon AppIcon \
  --include-all-app-icons \
  --enable-on-demand-resources NO \
  --output-partial-info-plist "$ICON_BUILD/IconInfo.plist" >/dev/null
cp "$ICON_BUILD/Assets.car" "$RESOURCES/Assets.car"

identity="$(
  security find-identity -v -p codesigning |
    awk -F '"' '/Developer ID Application/ { print $2; exit }'
)"
if [[ -z "$identity" ]]; then
  identity="$(
    security find-identity -v -p codesigning |
      awk -F '"' '/Apple Development/ { print $2; exit }'
  )"
fi
if [[ -z "$identity" ]]; then
  identity="-"
fi
if [[ -z "${APPLE_TEAM_ID:-}" && "$identity" =~ \(([A-Z0-9]+)\)$ ]]; then
  export APPLE_TEAM_ID="${BASH_REMATCH[1]}"
fi
if [[ "$notarize" -eq 1 && -z "${APPLE_TEAM_ID:-}" ]]; then
  echo "error: --notarize requires APPLE_TEAM_ID" >&2
  exit 64
fi
codesign_args=(--force --sign "$identity" --options runtime)
if [[ "$identity" != "-" ]]; then
  codesign_args+=(--timestamp)
fi

codesign "${codesign_args[@]}" --identifier com.automicvault.av "$ROOT/target/release/av"
codesign "${codesign_args[@]}" --identifier com.automicvault.av-brew-stub "$ROOT/target/release/av-brew-stub"
cp "$ROOT/target/release/av" "$MACOS/av"
cp "$ROOT/target/release/av-brew-stub" "$MACOS/av-brew-stub"
codesign "${codesign_args[@]}" --identifier com.automicvault.av "$MACOS/av"
codesign "${codesign_args[@]}" --identifier com.automicvault.av-brew-stub "$MACOS/av-brew-stub"
app_codesign_args=("${codesign_args[@]}")
if [[ -f "$MENU_HELPER_PROFILE" && "$identity" != "-" ]]; then
  cp "$MENU_HELPER_PROFILE" "$CONTENTS/embedded.provisionprofile"
  security cms -D -i "$MENU_HELPER_PROFILE" |
    plutil -extract Entitlements xml1 -o "$MENU_HELPER_ENTITLEMENTS" -
  # Existing releases stored items in the wildcard because it was the first
  # access group. Put the private group first so the app can migrate them.
  # Remove the wildcard after the migration release has been deployed.
  plutil -replace keychain-access-groups -json \
    "[\"${APPLE_TEAM_ID}.com.automicvault\",\"${APPLE_TEAM_ID}.*\"]" \
    "$MENU_HELPER_ENTITLEMENTS"
  app_codesign_args+=(--entitlements "$MENU_HELPER_ENTITLEMENTS")
fi
codesign "${app_codesign_args[@]}" "$APP"
if [[ "$dmg" -eq 1 ]]; then
  rm -rf "$DMG" "$DMG_STAGE"
  mkdir -p "$DMG_STAGE"
  ditto "$APP" "$DMG_STAGE/Automic Vault.app"
  create-dmg \
    --volname "Automic Vault" \
    --volicon "$ICON_BUILD/AppIcon.icns" \
    --window-size 500 300 \
    --icon "Automic Vault.app" 125 120 \
    --app-drop-link 375 120 \
    --codesign "$identity" \
    --overwrite \
    "$DMG" \
    "$DMG_STAGE"
  codesign --verify "$DMG"
  rm -rf "$DMG_STAGE"
  if [[ "$notarize" -eq 1 ]]; then
    "$ROOT/scripts/build-notarize-dmg.sh" "$DMG"
  fi
  if [[ "$publish" -eq 1 ]]; then
    publish_release "$VERSION" "$DMG"
  fi
fi
if [[ "$install" -eq 1 ]]; then
  install_app="$APP"
  if [[ "$dmg" -eq 1 ]]; then
    rm -rf "$DMG_MOUNT"
    mkdir -p "$DMG_MOUNT"
    hdiutil attach -nobrowse -readonly -mountpoint "$DMG_MOUNT" "$DMG"
    trap 'hdiutil detach "$DMG_MOUNT" >/dev/null 2>&1 || true' EXIT
    install_app="$DMG_MOUNT/Automic Vault.app"
  fi
  rm -rf "$INSTALLED_APP"
  ditto "$install_app" "$INSTALLED_APP"
  if [[ "$dmg" -eq 1 ]]; then
    hdiutil detach "$DMG_MOUNT"
    trap - EXIT
    rm -rf "$DMG_MOUNT"
  fi
  sudo install -m 0755 "$INSTALLED_APP/Contents/MacOS/av" /usr/local/bin/av
  mkdir -p "$HOME/Library/LaunchAgents"
  cp "$INSTALLED_APP/Contents/Library/LaunchAgents/$LAUNCH_AGENT_NAME.plist" "$INSTALLED_LAUNCH_AGENT"
  launchctl bootout "gui/$(id -u)" "$INSTALLED_LAUNCH_AGENT" 2>/dev/null || true
  launchctl bootstrap "gui/$(id -u)" "$INSTALLED_LAUNCH_AGENT"
  launchctl enable "gui/$(id -u)/$LAUNCH_AGENT_NAME"
  launchctl kickstart -k "gui/$(id -u)/$LAUNCH_AGENT_NAME"
  if [[ "$run" -eq 1 ]]; then
    pkill -x AutomicVaultMenubar || true
    open -n "$INSTALLED_APP"
  fi
elif [[ "$run" -eq 1 ]]; then
  pkill -x AutomicVaultMenubar || true
  open -n "$APP"
fi
if [[ "$dmg" -eq 1 ]]; then
  echo "$DMG"
else
  echo "$APP"
fi
