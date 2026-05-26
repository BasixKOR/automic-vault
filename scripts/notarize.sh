#!/bin/sh

/usr/bin/xcrun notarytool submit \
  --apple-id "${APPLE_USERNAME}" \
  --team-id "${team_id}" \
  --password "${APPLE_PASSWORD}" \
  --wait \
  "${final_dmg}" \
  >&2
