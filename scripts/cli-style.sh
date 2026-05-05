#!/usr/bin/env bash

cli_style_init() {
  CLI_NAME="${1:-Automic Vault}"
  CLI_USE_COLOR=0
  CLI_USE_UI=0

  if [[ -t 2 && -z "${NO_COLOR:-}" && "${TERM:-}" != "dumb" ]]; then
    CLI_USE_COLOR=1
    CLI_USE_UI=1
  fi

  if [[ "$CLI_USE_COLOR" == "1" ]]; then
    CLI_RESET=$'\033[0m'
    CLI_BOLD=$'\033[1m'
    CLI_DIM=$'\033[2m'
    CLI_RED=$'\033[31m'
    CLI_GREEN=$'\033[32m'
    CLI_YELLOW=$'\033[33m'
    CLI_BLUE=$'\033[34m'
    CLI_MAGENTA=$'\033[35m'
  else
    CLI_RESET=""
    CLI_BOLD=""
    CLI_DIM=""
    CLI_RED=""
    CLI_GREEN=""
    CLI_YELLOW=""
    CLI_BLUE=""
    CLI_MAGENTA=""
  fi

  if [[ "$CLI_USE_UI" == "1" ]]; then
    CLI_OK="✓"
    CLI_ERR="✗"
    CLI_WARN="!"
    CLI_STEP="◆"
    CLI_INFO="•"
  else
    CLI_OK="OK"
    CLI_ERR="ERROR"
    CLI_WARN="WARN"
    CLI_STEP="*"
    CLI_INFO="-"
  fi
}

cli_title() {
  local title="$1"
  if [[ "${CLI_USE_UI:-0}" == "1" ]]; then
    printf '%s╭─ %s%s%s\n' "$CLI_MAGENTA" "$CLI_BOLD" "$title" "$CLI_RESET" >&2
  else
    printf '%s\n' "$title" >&2
  fi
}

cli_done() {
  local message="$1"
  if [[ "${CLI_USE_UI:-0}" == "1" ]]; then
    printf '%s╰─ %s%s %s%s\n' \
      "$CLI_MAGENTA" "$CLI_GREEN" "$CLI_OK" "$message" "$CLI_RESET" >&2
  else
    printf 'OK %s\n' "$message" >&2
  fi
}

cli_step() {
  local message="$1"
  if [[ "${CLI_USE_UI:-0}" == "1" ]]; then
    printf '%s│%s %s%s %s%s\n' \
      "$CLI_MAGENTA" "$CLI_RESET" "$CLI_BLUE" "$CLI_STEP" "$message" "$CLI_RESET" >&2
  else
    printf '* %s\n' "$message" >&2
  fi
}

cli_info() {
  local message="$1"
  if [[ "${CLI_USE_UI:-0}" == "1" ]]; then
    printf '%s│%s %s%s %s%s\n' \
      "$CLI_MAGENTA" "$CLI_RESET" "$CLI_DIM" "$CLI_INFO" "$message" "$CLI_RESET" >&2
  else
    printf '%s %s\n' "-" "$message" >&2
  fi
}

cli_warn() {
  local message="$1"
  if [[ "${CLI_USE_UI:-0}" == "1" ]]; then
    printf '%s│%s %s%s %s%s\n' \
      "$CLI_MAGENTA" "$CLI_RESET" "$CLI_YELLOW" "$CLI_WARN" "$message" "$CLI_RESET" >&2
  else
    printf 'WARN %s\n' "$message" >&2
  fi
}

cli_error() {
  local message="$1"
  if [[ "${CLI_USE_UI:-0}" == "1" ]]; then
    printf '%s│%s %s%s %s%s\n' \
      "$CLI_MAGENTA" "$CLI_RESET" "$CLI_RED" "$CLI_ERR" "$message" "$CLI_RESET" >&2
  else
    printf 'ERROR %s\n' "$message" >&2
  fi
}

cli_die() {
  cli_error "$1"
  exit 1
}

cli_require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    cli_die "${tool} is required"
  fi
}
