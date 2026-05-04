#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
source "${repo_root}/scripts/cli-style.sh"
cli_style_init "Automic Vault"

background=false

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
cli_step "Preparing local app bundle"
app_path="$("${repo_root}/scripts/build-app.sh")"
cli_info "App: ${app_path}"

if [[ "${background}" == "true" ]]; then
  cli_step "Launching app"
  "${app_path}/Contents/MacOS/Automic Vault" &
  wait $!
fi

cli_step "Launching app"
exec "${app_path}/Contents/MacOS/Automic Vault"
