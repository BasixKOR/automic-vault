#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
AV=/usr/local/bin/av
APP_AV="/Applications/Automic Vault.app/Contents/MacOS/av"
RELEASE_REPOSITORY=https://github.com/automic-vault/automic-vault
HARDENERS=(terraform opentofu oxide-cli goat railway ordercli uaa-cli openhue-cli plumber)
COMMANDS=(terraform tofu oxide goat railway ordercli uaa openhue plumber)
ASSETS=(
  OpenTofu-Isotope-darwin-arm64.tgz
  Oxide-CLI-Isotope-darwin-arm64.tgz
  goat-Isotope-darwin-arm64.tgz
  Railway-Isotope-darwin-arm64.tgz
  ordercli-Isotope-darwin-arm64.tgz
  UAA-CLI-Isotope-darwin-arm64.tgz
  OpenHue-CLI-Isotope-darwin-arm64.tgz
  Plumber-Isotope-darwin-arm64.tgz
)

[[ "$(uname -s)" == Darwin ]] || { echo "error: macOS is required" >&2; exit 1; }
[[ "$(id -u)" -ne 0 ]] || {
  echo "error: run this script as your normal user, not with sudo" >&2
  exit 1
}
[[ -x "$AV" && -x "$APP_AV" ]] || {
  echo "error: install and open the current Automic Vault app first" >&2
  exit 1
}
codesign --verify --strict "$AV"
[[ "$("$AV" __version)" == "$("$APP_AV" __version)" ]] || {
  echo "error: /usr/local/bin/av does not match the installed app revision" >&2
  exit 1
}
launchctl print "gui/$(id -u)/com.automicvault.menubar-helper" >/dev/null || {
  echo "error: open Automic Vault before running the smoke test" >&2
  exit 1
}

version="$(awk -F '"' '/^version = / { print $2; exit }' "$ROOT/Cargo.toml")"
base="$RELEASE_REPOSITORY/releases/download/$version"
work="$(mktemp -d "${TMPDIR:-/tmp}/av-hardener-smoke.XXXXXX")"
keeper_pid=
cleanup() {
  if [[ -n "$keeper_pid" ]]; then kill "$keeper_pid" 2>/dev/null || true; fi
  rm -rf "$work"
}
trap cleanup EXIT INT TERM

echo "Preflighting Automic Vault $version release assets…"
curl --fail --location --silent --show-error "$base/SHA256SUMS" --output "$work/SHA256SUMS"
for asset in "${ASSETS[@]}"; do
  matches="$(awk -v asset="$asset" '$2 == asset { count++ } END { print count + 0 }' "$work/SHA256SUMS")"
  [[ "$matches" == 1 ]] || {
    echo "error: $asset is not uniquely listed in $base/SHA256SUMS" >&2
    exit 1
  }
  curl --fail --location --silent --show-error --head "$base/$asset" >/dev/null
done

echo "Authenticating once for protected Target installation…"
sudo -v
parent_pid=$$
(
  # Keep the one authentication valid while each hardener performs its own
  # narrowly scoped privileged installation.
  while kill -0 "$parent_pid" 2>/dev/null; do
    sudo -n -v || exit
    sleep 45
  done
) &
keeper_pid=$!

for hardener in "${HARDENERS[@]}"; do
  echo
  echo "===== av harden $hardener ====="
  "$AV" harden "$hardener" --yes
done

echo
echo "Verifying Hardened State, signatures, and command resolution…"
for index in "${!HARDENERS[@]}"; do
  hardener="${HARDENERS[$index]}"
  command="${COMMANDS[$index]}"
  target="$(command -v "$command")"
  codesign --verify --strict "$target"
  details="$(codesign -d -vvv "$target" 2>&1)"
  [[ "$details" == *"flags=0x10000(runtime)"* ]] || {
    echo "error: $target lacks Hardened Runtime" >&2
    exit 1
  }
  entitlements="$(codesign -d --entitlements :- "$target" 2>/dev/null)"
  [[ -z "$entitlements" ]] || {
    echo "error: $target has unexpected entitlements" >&2
    exit 1
  }
  "$AV" doctor "$hardener" --json >/dev/null
  printf 'PASS  %-12s %s\n' "$hardener" "$target"
done

echo
echo "Rerunning hardeners to verify idempotence…"
for hardener in "${HARDENERS[@]}"; do
  "$AV" harden "$hardener" --yes >/dev/null
done

echo "PASS: all new hardeners installed, verified, and remained idempotent"
