#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
AV_ROOT="$(cd "$ROOT/../.." && pwd)"
APP="$ROOT/build/Automic Vault.app"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"

cargo build --release --manifest-path "$AV_ROOT/Cargo.toml"
swift build -c release --package-path "$ROOT"

rm -rf "$APP"
mkdir -p "$MACOS" "$RESOURCES"
cp "$ROOT/.build/release/AutomicVaultMenubar" "$MACOS/AutomicVaultMenubar"
cp "$ROOT/Info.plist" "$CONTENTS/Info.plist"
cp "$ROOT/Resources/NSMenuItem.png" "$RESOURCES/NSMenuItem.png"
cp "$ROOT/Resources/AppIcon.icns" "$RESOURCES/AppIcon.icns"

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

codesign --force --sign "$identity" --identifier com.automicvault.av "$AV_ROOT/target/release/av"
cp "$AV_ROOT/target/release/av" "$MACOS/av"
codesign --force --sign "$identity" --identifier com.automicvault.av "$MACOS/av"
codesign --force --sign "$identity" "$APP"
echo "$APP"
