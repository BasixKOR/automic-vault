#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DESTINATION="${1:?usage: build-plumber-isotope.sh DESTINATION}"
VERSION="$(tr -d '[:space:]' <"$ROOT/isotopes/plumber.version")"
EXPECTED_SHA256="$(tr -d '[:space:]' <"$ROOT/isotopes/plumber.source-sha256")"
COMMIT="$(tr -d '[:space:]' <"$ROOT/isotopes/plumber.commit")"
TOOLCHAIN="${PLUMBER_GO_TOOLCHAIN:-go1.25.5}"
SOURCE="source.tar.gz"
BASE="https://github.com/streamdal/plumber/archive"
OUTPUT="$DESTINATION/Plumber-Isotope-darwin-arm64.tgz"
WORK="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/plumber-isotope.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
[[ "$EXPECTED_SHA256" =~ ^[0-9a-f]{64}$ ]]
[[ "$COMMIT" =~ ^[0-9a-f]{40}$ ]]
mkdir -p "$DESTINATION" "$WORK/source" "$WORK/bin"
curl --fail --location --silent --show-error "$BASE/$COMMIT.tar.gz" --output "$WORK/$SOURCE"
printf '%s  %s\n' "$EXPECTED_SHA256" "$WORK/$SOURCE" | shasum -a 256 --check

prefix="plumber-${COMMIT}/"
tar -tzf "$WORK/$SOURCE" | awk -v prefix="$prefix" '
  index($0, prefix) != 1 || $0 ~ /(^|\/)\.\.($|\/)/ { exit 1 }
'
if tar -tvzf "$WORK/$SOURCE" | awk '$1 ~ /^[lh]/ { found = 1 } END { exit !found }'; then
  echo "Plumber source archive contains links." >&2
  exit 1
fi
tar -xzf "$WORK/$SOURCE" --strip-components 1 -C "$WORK/source"
git -C "$WORK/source" init --quiet
git -C "$WORK/source" remote add origin https://github.com/streamdal/plumber.git
git -C "$WORK/source" fetch --quiet --depth 1 origin "$COMMIT"
git -C "$WORK/source" update-ref HEAD "$COMMIT"
git -C "$WORK/source" read-tree "$COMMIT"
git -C "$WORK/source" diff --exit-code
[[ -z "$(git -C "$WORK/source" ls-files --others --exclude-standard)" ]]
patch --batch --fuzz=0 -d "$WORK/source" -p1 <"$ROOT/isotopes/plumber.patch"

(
  cd "$WORK/source"
  GOTOOLCHAIN="$TOOLCHAIN" CGO_ENABLED=0 go test -mod=vendor ./config
  GOTOOLCHAIN="$TOOLCHAIN" CGO_ENABLED=0 go build \
    -mod=vendor -trimpath -buildvcs=true \
    -ldflags "-X github.com/streamdal/plumber/options.VERSION=v$VERSION" \
    -o "$WORK/bin/plumber" .
)
chmod 0755 "$WORK/bin/plumber"
build_info="$(go version -m "$WORK/bin/plumber")"
[[ "$build_info" == *"vcs.revision=$COMMIT"* && "$build_info" == *"vcs.modified=true"* ]]

identity="$(
  security find-identity -v -p codesigning |
    awk -F '"' '/Developer ID Application/ { print $2; exit }'
)"
[[ -n "$identity" ]]
codesign --force --sign "$identity" --options runtime --timestamp --identifier plumber "$WORK/bin/plumber"
requirement='=identifier "plumber" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = "ZU76A67LGU"'
codesign --verify --strict -R "$requirement" "$WORK/bin/plumber"
details="$(codesign -d -vvv "$WORK/bin/plumber" 2>&1)"
[[ "$details" == *'flags=0x10000(runtime)'* && "$details" == *'TeamIdentifier=ZU76A67LGU'* && "$details" == *'Timestamp='* ]]
entitlements="$(codesign -d --entitlements :- "$WORK/bin/plumber" 2>/dev/null)"
[[ -z "$entitlements" ]]
[[ "$("$WORK/bin/plumber" --version)" == "v$VERSION" ]]

COPYFILE_DISABLE=1 tar -czf "$OUTPUT" -C "$WORK" bin/plumber
[[ "$(tar -tzf "$OUTPUT")" == "bin/plumber" ]]
