#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DESTINATION="${1:?usage: build-opentofu-isotope.sh DESTINATION}"
VERSION="$(tr -d '[:space:]' <"$ROOT/isotopes/opentofu.version")"
ASSET="tofu_${VERSION}_darwin_arm64.zip"
SUMS="tofu_${VERSION}_SHA256SUMS"
BASE="https://github.com/opentofu/opentofu/releases/download/v${VERSION}"
IDENTITY="https://github.com/opentofu/opentofu/.github/workflows/release.yml@refs/heads/v${VERSION%.*}"
OUTPUT="$DESTINATION/OpenTofu-Isotope-darwin-arm64.tgz"
WORK="$(mktemp -d "$RUNNER_TEMP/opentofu-isotope.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
mkdir -p "$DESTINATION" "$WORK/bin"
for name in "$ASSET" "$SUMS" "$SUMS.sig" "$SUMS.pem"; do
  curl --fail --location --silent --show-error "$BASE/$name" --output "$WORK/$name"
done

cosign verify-blob \
  --certificate-identity "$IDENTITY" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate "$WORK/$SUMS.pem" \
  --signature "$WORK/$SUMS.sig" \
  "$WORK/$SUMS"
(
  cd "$WORK"
  checksum_line="$(awk -v name="$ASSET" '$2 == name { print }' "$SUMS")"
  [[ "$(printf '%s\n' "$checksum_line" | wc -l | tr -d ' ')" == 1 ]]
  printf '%s\n' "$checksum_line" | shasum -a 256 --check
)

listing="$(unzip -Z1 "$WORK/$ASSET" | LC_ALL=C sort)"
[[ "$listing" == $'CHANGELOG.md\nLICENSE\nREADME.md\ntofu' ]]
unzip -p "$WORK/$ASSET" tofu >"$WORK/bin/tofu"
chmod 0755 "$WORK/bin/tofu"

identity="$(
  security find-identity -v -p codesigning |
    awk -F '"' '/Developer ID Application/ { print $2; exit }'
)"
[[ -n "$identity" ]]
codesign --force --sign "$identity" --options runtime --timestamp --identifier tofu "$WORK/bin/tofu"
requirement='=identifier "tofu" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = "ZU76A67LGU"'
codesign --verify --strict -R "$requirement" "$WORK/bin/tofu"
details="$(codesign -d -vvv "$WORK/bin/tofu" 2>&1)"
[[ "$details" == *'flags=0x10000(runtime)'* && "$details" == *'TeamIdentifier=ZU76A67LGU'* && "$details" == *'Timestamp='* ]]
entitlements="$(codesign -d --entitlements :- "$WORK/bin/tofu" 2>/dev/null)"
[[ -z "$entitlements" ]]

COPYFILE_DISABLE=1 tar -czf "$OUTPUT" -C "$WORK" bin/tofu
[[ "$(tar -tzf "$OUTPUT")" == "bin/tofu" ]]
