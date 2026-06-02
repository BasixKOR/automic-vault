#!/usr/bin/env bash

set -euo pipefail

scanner_url="${AUTOMIC_VAULT_SCANNER_URL:-https://www.automicvault.com/scanner.gz}"
install_url="${AUTOMIC_VAULT_INSTALL_URL:-https://www.automicvault.com/install.sh}"
tmp_dir=""

use_color=false
if [[ -t 2 && -z "${NO_COLOR:-}" && "${TERM:-}" != "dumb" ]]; then
  use_color=true
fi

if [[ "${use_color}" == true ]]; then
  reset=$'\033[0m'
  bold=$'\033[1m'
  dim=$'\033[2m'
  red=$'\033[31m'
  green=$'\033[32m'
  blue=$'\033[34m'
  magenta=$'\033[35m'
  glyph_step="◆"
  glyph_ok="✓"
  glyph_error="✗"
else
  reset=""
  bold=""
  dim=""
  red=""
  green=""
  blue=""
  magenta=""
  glyph_step="*"
  glyph_ok="OK"
  glyph_error="ERROR"
fi

log() {
  printf '%s\n' "$*" >&2
}

title() {
  if [[ "${use_color}" == true ]]; then
    log "${magenta}╭─ ${bold}Automic Vault scanner${reset}"
    log "${magenta}│${reset} ${dim}detector-only secret exposure check${reset}"
  else
    log "Automic Vault scanner"
    log "detector-only secret exposure check"
  fi
}

step() {
  if [[ "${use_color}" == true ]]; then
    log "${magenta}│${reset} ${blue}${glyph_step}${reset} $*"
  else
    log "${glyph_step} $*"
  fi
}

ok() {
  if [[ "${use_color}" == true ]]; then
    log "${magenta}│${reset} ${green}${glyph_ok}${reset} $*"
  else
    log "${glyph_ok} $*"
  fi
}

error() {
  if [[ "${use_color}" == true ]]; then
    log "${magenta}│${reset} ${red}${glyph_error}${reset} $*"
  else
    log "${glyph_error} $*"
  fi
}

done_line() {
  if [[ "${use_color}" == true ]]; then
    log "${magenta}╰─${reset} ${green}${glyph_ok}${reset} $*"
  else
    log "${glyph_ok} $*"
  fi
}

separator_line() {
  if [[ "${use_color}" == true ]]; then
    log "${magenta}│${reset}"
  else
    log ""
  fi
}

die() {
  error "$*"
  exit 1
}

cleanup() {
  if [[ -n "${tmp_dir}" && -d "${tmp_dir}" ]]; then
    /bin/rm -rf "${tmp_dir}"
  fi
}

require_executable() {
  local path="$1"
  local label="$2"
  if [[ ! -x "${path}" ]]; then
    die "${label} is required at ${path}."
  fi
}

show_install_recommendation() {
  local arg
  for arg in "$@"; do
    case "${arg}" in
      --json|--jsonl|--help|-h|--version|-V)
        return 1
        ;;
    esac
  done
  return 0
}

recommend_install() {
  local scan_output_path="$1"
  local scan_output
  local command_line

  command_line="/usr/bin/curl -fsSL ${install_url} | /bin/bash"
  scan_output="$(<"${scan_output_path}")"

  log ""
  if [[ "${scan_output}" == *"Findings"* ]]; then
    log "${bold}Fix these findings with Automic Vault.${reset}"
    log "${dim}Move exposed credentials out of plaintext and inject them only when trusted tools run.${reset}"
  else
    log "${bold}Keep it clean with Automic Vault.${reset}"
    log "${dim}Store secrets safely now so future agent runs stay away from plaintext credentials.${reset}"
  fi
  log "  ${blue}${command_line}${reset}"
}

sandbox_literal() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  printf '%s' "${value}"
}

write_sandbox_profile() {
  local scanner_path="$1"
  local profile_path="$2"
  local scanner_real_path
  local scanner_path_literal
  local scanner_real_path_literal

  scanner_real_path="$(cd "$(dirname "${scanner_path}")" && pwd -P)/$(basename "${scanner_path}")"
  scanner_path_literal="$(sandbox_literal "${scanner_path}")"
  scanner_real_path_literal="$(sandbox_literal "${scanner_real_path}")"
  cat >"${profile_path}" <<EOF
(version 1)
(allow default)
(deny network*)
(deny file-write*)
(deny process-fork)
(deny process-exec)
(allow process-exec (literal "${scanner_path_literal}"))
(allow process-exec (literal "${scanner_real_path_literal}"))
EOF
}

trap cleanup EXIT

title

if [[ "$(/usr/bin/uname -s)" != "Darwin" ]]; then
  die "The sandboxed scanner currently runs on macOS."
fi

require_executable "/usr/bin/curl" "curl"
require_executable "/usr/bin/gzip" "gzip"
require_executable "/usr/bin/sandbox-exec" "sandbox-exec"
require_executable "/usr/bin/tee" "tee"

tmp_dir="$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/av-scanner.XXXXXX")"
payload_path="${tmp_dir}/scanner.gz"
scanner_path="${tmp_dir}/scanner"
sandbox_profile_path="${tmp_dir}/scanner.sb"
scanner_output_path="${tmp_dir}/scanner.out"

step "Downloading scanner"
if ! /usr/bin/curl -fsSL "${scanner_url}" -o "${payload_path}"; then
  die "Could not download ${scanner_url}."
fi

step "Unpacking scanner"
if ! /usr/bin/gzip -dc "${payload_path}" >"${scanner_path}"; then
  die "Downloaded scanner was not a valid gzip file."
fi
/bin/chmod 755 "${scanner_path}"

write_sandbox_profile "${scanner_path}" "${sandbox_profile_path}"

ok "Writes denied"
ok "Network denied"
step "Running package-specific detectors"

if show_install_recommendation "$@"; then
  if AUTOMIC_VAULT_SCANNER_WRAPPER_UI=1 \
    /usr/bin/sandbox-exec -f "${sandbox_profile_path}" "${scanner_path}" "$@" </dev/null \
    | /usr/bin/tee "${scanner_output_path}"; then
    separator_line
    done_line "Scan complete"
    recommend_install "${scanner_output_path}"
  else
    status="$?"
    error "Scanner exited with status ${status}."
    exit "${status}"
  fi
elif AUTOMIC_VAULT_SCANNER_WRAPPER_UI=1 \
  /usr/bin/sandbox-exec -f "${sandbox_profile_path}" "${scanner_path}" "$@" </dev/null; then
  separator_line
  done_line "Scan complete"
else
  status="$?"
  error "Scanner exited with status ${status}."
  exit "${status}"
fi
