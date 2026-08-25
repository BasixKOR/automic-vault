#!/bin/bash
set -euo pipefail

AV=/usr/local/bin/av
APP_AV="/Applications/Automic Vault.app/Contents/MacOS/av"
FORMULA_ROOT=https://raw.githubusercontent.com/automic-vault/homebrew-isotopes/main/Formula
HARDENERS=(terraform opentofu oxide-cli goat railway ordercli uaa-cli openhue-cli plumber)
COMMANDS=(terraform tofu oxide goat railway ordercli uaa openhue plumber)
FORMULAS=(opentofu oxide.rs goat railway-cli ordercli uaa-cli openhue-cli plumber)
REPOSITORIES=(opentofu oxide.rs goat railway-cli ordercli uaa-cli openhue-cli plumber)

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

work="$(mktemp -d "${TMPDIR:-/tmp}/av-hardener-smoke.XXXXXX")"
keeper_pid=
cleanup() {
  if [[ -n "$keeper_pid" ]]; then kill "$keeper_pid" 2>/dev/null || true; fi
  rm -rf "$work"
}
trap cleanup EXIT INT TERM

echo "Preflighting signed Isotope fork releases…"
for index in "${!FORMULAS[@]}"; do
  formula="${FORMULAS[$index]}"
  repository="${REPOSITORIES[$index]}"
  manifest="$work/$formula.rb"
  curl --fail --location --silent --show-error "$FORMULA_ROOT/$formula.rb" --output "$manifest"
  url="$(awk -F '"' '/^[[:space:]]*url "[^"]+"[[:space:]]*$/ { print $2 }' "$manifest")"
  sha256="$(awk -F '"' '/^[[:space:]]*sha256 "[^"]+"[[:space:]]*$/ { print $2 }' "$manifest")"
  [[ -n "$url" && "$url" != *$'\n'* ]] || {
    echo "error: $formula formula must contain exactly one release URL" >&2
    exit 1
  }
  prefix="https://github.com/automic-vault/$repository/releases/download/"
  [[ "$url" == "$prefix"* && "$url" == *.tgz ]] || {
    echo "error: $formula formula points outside $repository releases" >&2
    exit 1
  }
  [[ "$sha256" =~ ^[0-9a-f]{64}$ ]] || {
    echo "error: $formula formula must contain exactly one SHA-256 digest" >&2
    exit 1
  }
  curl --fail --location --silent --show-error --head "$url" >/dev/null
  printf 'PASS  %-12s %s\n' "$formula" "$url"
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
