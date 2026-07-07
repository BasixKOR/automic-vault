#!/usr/bin/env bash
set -euo pipefail

run=0
install=0
dmg=0
notarize=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --run) run=1 ;;
    --install) install=1 ;;
    --dmg) dmg=1 ;;
    --notarize) notarize=1 ;;
    *)
      echo "usage: $0 [--run] [--install] [--dmg] [--notarize]" >&2
      exit 64
      ;;
  esac
  shift
done
if [[ "$notarize" -eq 1 && "$dmg" -ne 1 ]]; then
  echo "error: --notarize requires --dmg" >&2
  exit 64
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MENU_HELPER="$ROOT/src/menu-helper"
SWIFT_TARGET="$ROOT/target/swift"
APP="$SWIFT_TARGET/Automic Vault.app"
DMG="$SWIFT_TARGET/Automic Vault.dmg"
DMG_STAGE="$SWIFT_TARGET/dmg"
DMG_MOUNT="$SWIFT_TARGET/dmg-mount"
MENU_HELPER_PROFILE="$HOME/Library/MobileDevice/Provisioning Profiles/Automic_Vault_Menu_Developer_ID.provisionprofile"
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

rm -rf "$APP"
mkdir -p "$MACOS" "$RESOURCES" "$LAUNCH_AGENTS"
cp "$SWIFT_BIN/AutomicVaultMenubar" "$MACOS/AutomicVaultMenubar"
cp "$MENU_HELPER/Info.plist" "$CONTENTS/Info.plist"
cp "$MENU_HELPER/LaunchAgent.plist" "$LAUNCH_AGENT_PLIST"
cp "$MENU_HELPER/Resources/NSMenuItem.png" "$RESOURCES/NSMenuItem.png"
cp "$MENU_HELPER/Resources/AppIcon.icns" "$RESOURCES/AppIcon.icns"

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
cp "$ROOT/target/release/av" "$MACOS/av"
codesign "${codesign_args[@]}" --identifier com.automicvault.av "$MACOS/av"
app_codesign_args=("${codesign_args[@]}")
if [[ -f "$MENU_HELPER_PROFILE" && "$identity" != "-" ]]; then
  cp "$MENU_HELPER_PROFILE" "$CONTENTS/embedded.provisionprofile"
  security cms -D -i "$MENU_HELPER_PROFILE" |
    plutil -extract Entitlements xml1 -o "$MENU_HELPER_ENTITLEMENTS" -
  app_codesign_args+=(--entitlements "$MENU_HELPER_ENTITLEMENTS")
fi
codesign "${app_codesign_args[@]}" "$APP"
if [[ "$dmg" -eq 1 ]]; then
  rm -rf "$DMG" "$DMG_STAGE"
  mkdir -p "$DMG_STAGE"
  ditto "$APP" "$DMG_STAGE/Automic Vault.app"
  create-dmg \
    --volname "Automic Vault" \
    --volicon "$RESOURCES/AppIcon.icns" \
    --icon "Automic Vault.app" 125 120 \
    --app-drop-link 425 120 \
    --codesign "$identity" \
    --overwrite \
    "$DMG" \
    "$DMG_STAGE"
  codesign --verify "$DMG"
  rm -rf "$DMG_STAGE"
  if [[ "$notarize" -eq 1 ]]; then
    "$ROOT/scripts/build-notarize-dmg.sh" "$DMG"
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
