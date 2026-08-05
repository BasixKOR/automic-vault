#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUTPUT="${1:-$ROOT/target/scanner.tgz}"
IDENTITY="${SCANNER_CODESIGN_IDENTITY:--}"
TARGET="$ROOT/target/scanner-build"
SCANNER="$TARGET/release/scanner"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/av-scanner-package.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

MACOSX_DEPLOYMENT_TARGET=14.0 \
CARGO_TARGET_DIR="$TARGET" \
CARGO_PROFILE_RELEASE_OPT_LEVEL=z \
CARGO_PROFILE_RELEASE_LTO=fat \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
CARGO_PROFILE_RELEASE_STRIP=symbols \
CARGO_PROFILE_RELEASE_PANIC=abort \
RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-Wl,-dead_strip" \
  cargo build --release --locked --bin scanner --manifest-path "$ROOT/Cargo.toml"

codesign_args=(--force --sign "$IDENTITY" --options runtime)
if [[ "$IDENTITY" != "-" ]]; then
  codesign_args+=(--timestamp)
fi
codesign "${codesign_args[@]}" --identifier com.automicvault.scanner "$SCANNER"
codesign --verify --strict "$SCANNER"

cp "$SCANNER" "$TMP/scanner"
mkdir -p "$(dirname "$OUTPUT")"
tar -czf "$OUTPUT" -C "$TMP" scanner
printf '%s\n' "$OUTPUT"
