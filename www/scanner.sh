#!/usr/bin/env bash

set -euo pipefail

scanner_url="${AUTOMIC_VAULT_SCANNER_URL:-https://www.automicvault.com/scanner.gz}"
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

tmp_dir="$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/av-scanner.XXXXXX")"
payload_path="${tmp_dir}/scanner.gz"
scanner_path="${tmp_dir}/scanner"
sandbox_profile_path="${tmp_dir}/scanner.sb"

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

if AUTOMIC_VAULT_SCANNER_WRAPPER_UI=1 \
  /usr/bin/sandbox-exec -f "${sandbox_profile_path}" "${scanner_path}" "$@" </dev/null; then
  done_line "Scan complete"
else
  status="$?"
  error "Scanner exited with status ${status}."
  exit "${status}"
fi
