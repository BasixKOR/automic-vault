#!/usr/local/bin/av inject +APPLE_PASSWORD /bin/sh

/usr/bin/xcrun notarytool submit \
  --apple-id "${APPLE_USERNAME}" \
  --team-id "${team_id}" \
  --password "${APPLE_PASSWORD}" \
  --wait \
  "$1" \
  >&2
