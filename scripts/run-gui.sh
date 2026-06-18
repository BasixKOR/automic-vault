#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
source "${repo_root}/scripts/cli-style.sh"
cli_style_init "Automic Vault"

background=false

terminate_existing_app() {
  cli_step "Stopping existing app instances"

  /usr/bin/osascript \
    -e 'tell application id "com.automicvault" to quit' \
    >/dev/null 2>&1 || true
  /usr/bin/osascript \
    -e 'tell application id "com.automicvault.menu-helper" to quit' \
    >/dev/null 2>&1 || true

  local deadline=$((SECONDS + 5))
  while pgrep -f \
    'Automic Vault(\.app)?/Contents/MacOS/Automic Vault|Automic Vault Menu(\.app)?/Contents/MacOS/Automic Vault Menu' \
    >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      pkill -TERM -f \
        'Automic Vault(\.app)?/Contents/MacOS/Automic Vault|Automic Vault Menu(\.app)?/Contents/MacOS/Automic Vault Menu' \
        >/dev/null 2>&1 || true
      break
    fi
    sleep 0.1
  done

  pkill -TERM -f \
    'Automic Vault\.app/Contents/Resources/av serve|Automic Vault Menu\.app/Contents/Resources/av serve' \
    >/dev/null 2>&1 || true
  rm -rf \
    "${HOME}/Library/Saved Application State/com.automicvault.savedState"
  rm -f "${HOME}/Library/Application Support/Automic Vault/nucleus.sock"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --background)
      background=true
      shift
      ;;
    --help|-h)
      echo "Usage: $0 [--background]"
      exit 0
      ;;
    *)
      cli_error "Unknown argument: $1"
      echo "Usage: $0 [--background]" >&2
      exit 1
      ;;
  esac
done

cli_title "Run Automic Vault"
terminate_existing_app
cli_step "Preparing local app bundle"
app_path="$("${repo_root}/scripts/build-app.sh")"
cli_info "App: ${app_path}"

if [[ "${background}" == "true" ]]; then
  cli_step "Launching app"
  nohup "${app_path}/Contents/MacOS/Automic Vault" \
    >/tmp/automic-vault-gui.log 2>&1 &
  exit 0
fi

cli_step "Launching app"
exec "${app_path}/Contents/MacOS/Automic Vault"
