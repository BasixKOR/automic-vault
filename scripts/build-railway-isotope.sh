#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DESTINATION="${1:?usage: build-railway-isotope.sh DESTINATION}"
VERSION="$(tr -d '[:space:]' <"$ROOT/isotopes/railway.version")"
EXPECTED_SHA256="$(tr -d '[:space:]' <"$ROOT/isotopes/railway.source-sha256")"
COMMIT="$(tr -d '[:space:]' <"$ROOT/isotopes/railway.commit")"
TOOLCHAIN="${RAILWAY_RUST_TOOLCHAIN:-1.96.0}"
SOURCE="source.tar.gz"
BASE="https://github.com/railwayapp/cli/archive/refs/tags"
OUTPUT="$DESTINATION/Railway-Isotope-darwin-arm64.tgz"
WORK="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/railway-isotope.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
[[ "$EXPECTED_SHA256" =~ ^[0-9a-f]{64}$ ]]
[[ "$COMMIT" =~ ^[0-9a-f]{40}$ ]]
mkdir -p "$DESTINATION" "$WORK/source" "$WORK/bin"
curl --fail --location --silent --show-error "$BASE/v${VERSION}.tar.gz" --output "$WORK/$SOURCE"
printf '%s  %s\n' "$EXPECTED_SHA256" "$WORK/$SOURCE" | shasum -a 256 --check

prefix="cli-${VERSION}/"
tar -tzf "$WORK/$SOURCE" | awk -v prefix="$prefix" '
  index($0, prefix) != 1 || $0 ~ /(^|\/)\.\.($|\/)/ { exit 1 }
'
if tar -tvzf "$WORK/$SOURCE" | awk '$1 ~ /^[lh]/ { found = 1 } END { exit !found }'; then
  echo "Railway source archive contains links." >&2
  exit 1
fi
tar -xzf "$WORK/$SOURCE" --strip-components 1 -C "$WORK/source"
git -C "$WORK/source" init --quiet
git -C "$WORK/source" remote add origin https://github.com/railwayapp/cli.git
git -C "$WORK/source" fetch --quiet --depth 1 origin "$COMMIT"
git -C "$WORK/source" update-ref HEAD "$COMMIT"
git -C "$WORK/source" read-tree "$COMMIT"
git -C "$WORK/source" diff --exit-code
[[ -z "$(git -C "$WORK/source" ls-files --others --exclude-standard)" ]]
patch --batch --fuzz=0 -d "$WORK/source" -p1 <"$ROOT/isotopes/railway.patch"

(cd "$WORK/source" && cargo +"$TOOLCHAIN" build --locked --release --bin railway)
cp "$WORK/source/target/release/railway" "$WORK/bin/railway"
chmod 0755 "$WORK/bin/railway"

identity="$(
  security find-identity -v -p codesigning |
    awk -F '"' '/Developer ID Application/ { print $2; exit }'
)"
[[ -n "$identity" ]]
codesign --force --sign "$identity" --options runtime --timestamp --identifier railway "$WORK/bin/railway"
requirement='=identifier "railway" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = "ZU76A67LGU"'
codesign --verify --strict -R "$requirement" "$WORK/bin/railway"
details="$(codesign -d -vvv "$WORK/bin/railway" 2>&1)"
[[ "$details" == *'flags=0x10000(runtime)'* && "$details" == *'TeamIdentifier=ZU76A67LGU'* && "$details" == *'Timestamp='* ]]
entitlements="$(codesign -d --entitlements :- "$WORK/bin/railway" 2>/dev/null)"
[[ -z "$entitlements" ]]
[[ "$("$WORK/bin/railway" --version)" == "railway $VERSION" ]]

COPYFILE_DISABLE=1 tar -czf "$OUTPUT" -C "$WORK" bin/railway
[[ "$(tar -tzf "$OUTPUT")" == "bin/railway" ]]
