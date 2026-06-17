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

refresh_local_cli() {
  local app_path="$1"
  local bundled_av="${app_path}/Contents/Resources/av"
  local target="/usr/local/bin/av"
  local target_dir
  local temp_target

  [[ -x "${bundled_av}" ]] || return

  if [[ -e "${target}" ]]; then
    if [[ ! -w "${target}" ]]; then
      cli_warn "Skipping ${target}; file is not writable"
      return
    fi
  else
    target_dir="$(dirname "${target}")"
    if [[ ! -w "${target_dir}" ]]; then
      cli_warn "Skipping ${target}; ${target_dir} is not writable"
      return
    fi
  fi

  if cmp -s "${bundled_av}" "${target}" 2>/dev/null; then
    return
  fi

  cli_step "Refreshing local av CLI"
  target_dir="$(dirname "${target}")"
  temp_target="$(mktemp "${target_dir}/.av.XXXXXX")"
  if ! cp "${bundled_av}" "${temp_target}" \
    || ! chmod 755 "${temp_target}" \
    || ! mv -f "${temp_target}" "${target}"; then
    rm -f "${temp_target}"
    return 1
  fi
  cli_info "Updated ${target}"
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
refresh_local_cli "${app_path}"

if [[ "${background}" == "true" ]]; then
  cli_step "Launching app"
  nohup "${app_path}/Contents/MacOS/Automic Vault" \
    >/tmp/automic-vault-gui.log 2>&1 &
  exit 0
fi

cli_step "Launching app"
exec "${app_path}/Contents/MacOS/Automic Vault"
