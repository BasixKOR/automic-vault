#!/usr/local/bin/av inject +APPLE_PASSWORD +APPLE_USERNAME /bin/sh
set -eu

output="$(/usr/bin/xcrun notarytool submit \
  --apple-id "${APPLE_USERNAME}" \
  --team-id "${APPLE_TEAM_ID}" \
  --password "${APPLE_PASSWORD}" \
  --wait \
  "$1" \
  2>&1)"
printf '%s\n' "$output" >&2
printf '%s\n' "$output" | grep -q "status: Accepted"
/usr/bin/xcrun stapler staple "$1" >&2
/usr/bin/xcrun stapler validate "$1" >&2
