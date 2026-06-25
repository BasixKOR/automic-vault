#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MENU_HELPER="$ROOT/src/menu-helper"
APP="$MENU_HELPER/build/Automic Vault.app"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"

cargo build --release --manifest-path "$ROOT/Cargo.toml"
swift build -c release --package-path "$MENU_HELPER"

rm -rf "$APP"
mkdir -p "$MACOS" "$RESOURCES"
cp "$MENU_HELPER/.build/release/AutomicVaultMenubar" "$MACOS/AutomicVaultMenubar"
cp "$MENU_HELPER/Info.plist" "$CONTENTS/Info.plist"
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
cp "$ROOT/target/release/av" "$MACOS/av"
codesign --force --sign "$identity" --identifier com.automicvault.av "$MACOS/av"
codesign --force --sign "$identity" "$APP"
echo "$APP"
