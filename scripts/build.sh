#!/usr/bin/env bash
set -euo pipefail

run=0
install=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --run) run=1 ;;
    --install) install=1 ;;
    *)
      echo "usage: $0 [--run] [--install]" >&2
      exit 64
      ;;
  esac
  shift
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MENU_HELPER="$ROOT/src/menu-helper"
SWIFT_TARGET="$ROOT/target/swift"
APP="$SWIFT_TARGET/Automic Vault.app"
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

codesign --force --sign "$identity" --identifier com.automicvault.av "$ROOT/target/release/av"
if [[ "$install" -eq 1 ]]; then
  sudo install -m 0755 "$ROOT/target/release/av" /usr/local/bin/av
fi
cp "$ROOT/target/release/av" "$MACOS/av"
codesign --force --sign "$identity" --identifier com.automicvault.av "$MACOS/av"
codesign --force --sign "$identity" "$APP"
if [[ "$install" -eq 1 ]]; then
  sudo rm -rf "$INSTALLED_APP"
  sudo ditto "$APP" "$INSTALLED_APP"
  mkdir -p "$HOME/Library/LaunchAgents"
  cp "$LAUNCH_AGENT_PLIST" "$INSTALLED_LAUNCH_AGENT"
  launchctl bootout "gui/$(id -u)" "$INSTALLED_LAUNCH_AGENT" 2>/dev/null || true
  launchctl bootstrap "gui/$(id -u)" "$INSTALLED_LAUNCH_AGENT"
  launchctl enable "gui/$(id -u)/$LAUNCH_AGENT_NAME"
  launchctl kickstart -k "gui/$(id -u)/$LAUNCH_AGENT_NAME"
fi
if [[ "$run" -eq 1 ]]; then
  pkill -x AutomicVaultMenubar || true
  open -n "$APP"
fi
echo "$APP"
