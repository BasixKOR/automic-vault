#!/bin/sh

set -e

tmp="$(mktemp -d)"
app="$tmp/mnt/Automic Vault.app"

cleanup() {
  set +x
  /usr/bin/hdiutil detach "$tmp/mnt" -quiet 2>/dev/null || true
  /bin/rm -rf "$tmp"
}

trap cleanup EXIT

/usr/bin/curl -sSfL https://automicvault.com/av.dmg -o "$tmp/av.dmg"

/usr/bin/hdiutil attach "$tmp/av.dmg" \
  -mountpoint "$tmp/mnt" \
  -nobrowse \
  -readonly \
  -quiet

/usr/sbin/spctl -a -vv --type exec "$app"

/usr/bin/codesign --verify --deep --strict "$app"

/usr/bin/codesign -dv --verbose=4 "$app" 2>&1 \
  | /usr/bin/grep -q '^TeamIdentifier=ZU76A67LGU$'

/usr/bin/ditto "$app" "/Applications/Automic Vault.app"
/usr/bin/sudo /bin/mkdir -p /usr/local/bin
/usr/bin/sudo /usr/bin/install -m 755 \
  "/Applications/Automic Vault.app/Contents/Resources/av" \
  /usr/local/bin/av
