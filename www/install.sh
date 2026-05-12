#!/usr/bin/env bash

set -eo pipefail

tmp="$(mktemp -d)"

cleanup() {
  /usr/bin/hdiutil detach "$tmp/mnt" -quiet 2>/dev/null || true
  /bin/rm -rf "$tmp"
}

trap cleanup EXIT

set -x

/usr/bin/curl -sSfL https://automicvault.com/AutomicVault.dmg -o "$tmp/av.dmg"

/usr/sbin/spctl -a -vv --type open "$tmp/av.dmg"

/usr/bin/hdiutil attach "$tmp/av.dmg" \
  -mountpoint "$tmp/mnt" \
  -nobrowse \
  -quiet

app="$(find "$tmp/mnt" -maxdepth 1 -name '*.app' -print -quit)"

[[ -n "$app" ]]

/usr/bin/codesign -dv --verbose=4 "$app" 2>&1 \
  | /usr/bin/grep -q '^TeamIdentifier=ZU76A67LGU$'

/usr/bin/ditto "$app" "/Applications/$(basename "$app")"
