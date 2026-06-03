#!/bin/sh

set -e

expected_team_id="ZU76A67LGU"

tmp="$(mktemp -d)"

cleanup() {
  /usr/bin/hdiutil detach "$tmp/mnt" -quiet 2>/dev/null || true
  /bin/rm -rf "$tmp"
}

trap cleanup EXIT

set -x

/usr/bin/curl -sSfL https://automicvault.com/av.dmg -o "$tmp/av.dmg"

/usr/bin/hdiutil attach "$tmp/av.dmg" \
  -mountpoint "$tmp/mnt" \
  -nobrowse \
  -readonly \
  -quiet

app="$(/usr/bin/find "$tmp/mnt" -maxdepth 1 -name '*.app' -print -quit)"

[ -n "$app" ]

/usr/sbin/spctl -a -vv --type exec "$app"

/usr/bin/codesign --verify --deep --strict "$app"

/usr/bin/codesign -dv --verbose=4 "$app" 2>&1 \
  | /usr/bin/grep -q "^TeamIdentifier=${expected_team_id}$"

installed_app="/Applications/$(/usr/bin/basename "$app")"

/usr/bin/ditto "$app" "$installed_app"

av="$installed_app/Contents/Resources/av"

[ -x "$av" ]

/usr/bin/sudo /bin/mkdir -p /usr/local/bin
/usr/bin/sudo /usr/bin/install -m 755 "$av" /usr/local/bin/av
