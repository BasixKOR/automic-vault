#!/usr/bin/env bash

set -uo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
combined_path="${repo_root}/data/combined.json"
radioisotopes_dir="${repo_root}/data/radioisotopes"
cache_control="public, max-age=3600"
color_mode="auto"
isotope_args=()

usage() {
  cat <<'EOF'
Usage: scripts/update-db.sh [--skip-isotope-builds] [--once]
                            [--color auto|always|never] [--no-color]

Refresh isotope metadata, rebuild the Homebrew package database, rebuild
data/combined.json, and upload it as /db.json.

This script runs one update and exits. Use scripts/update-all for the
hourly database loop and daily package-page deploy cadence.

Options:
  --skip-isotope-builds       Pass --skip-builds to build-isotopes.sh.
  --once                      Run one update immediately and exit.
                              This is the default behavior.
  --color auto|always|never   Control terminal color output.
                              Defaults to auto.
  --no-color                  Disable terminal color output.
  --help                      Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --once)
      # update-db.sh is always immediate and one-shot.
      shift
      ;;
    --interval-seconds)
      echo "--interval-seconds moved to scripts/update-all." >&2
      usage >&2
      exit 1
      ;;
    --skip-isotope-builds)
      isotope_args+=(--skip-builds)
      shift
      ;;
    --color)
      if [[ $# -lt 2 ]]; then
        echo "--color requires auto, always, or never" >&2
        exit 1
      fi
      color_mode="$2"
      shift 2
      ;;
    --no-color)
      color_mode="never"
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

case "${color_mode}" in
  auto|always|never)
    ;;
  *)
    echo "--color must be auto, always, or never" >&2
    exit 1
    ;;
esac

use_color=false
if [[ "${color_mode}" == "always" ]]; then
  use_color=true
elif [[ "${color_mode}" == "auto" ]]; then
  if [[ -t 2 && -z "${NO_COLOR:-}" && "${TERM:-}" != "dumb" ]]; then
    use_color=true
  fi
fi

if [[ "${use_color}" == true ]]; then
  bold=$'\033[1m'
  dim=$'\033[2m'
  red=$'\033[31m'
  green=$'\033[32m'
  blue=$'\033[34m'
  yellow=$'\033[33m'
  reset=$'\033[0m'
  glyph_step="◆"
  glyph_ok="✓"
  glyph_warn="!"
  glyph_error="✗"
else
  bold=""
  dim=""
  red=""
  green=""
  blue=""
  yellow=""
  reset=""
  glyph_step=">"
  glyph_ok="OK"
  glyph_warn="WARN"
  glyph_error="ERROR"
fi

log() {
  local level="$1"
  local message="$2"
  local color glyph

  case "${level}" in
    INFO)
      color="${blue}"
      glyph="${glyph_step}"
      ;;
    OK)
      color="${green}"
      glyph="${glyph_ok}"
      ;;
    WARN)
      color="${yellow}"
      glyph="${glyph_warn}"
      ;;
    ERROR)
      color="${red}"
      glyph="${glyph_error}"
      ;;
    *)
      color=""
      glyph="${level}"
      ;;
  esac

  if [[ "${use_color}" == true ]]; then
    printf '%s %s%s%s %s%s%s %s\n' \
      "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" \
      "${color}" \
      "${glyph}" \
      "${reset}" \
      "${dim}" \
      "${level}" \
      "${reset}" \
      "${message}" >&2
  else
    printf '[%s] %-5s %s\n' \
      "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" \
      "${level}" \
      "${message}" >&2
  fi
}

log_header() {
  log INFO "${bold}Publishing combined database${reset}"
  log INFO "${dim}data/combined.json -> s3://${WWW_BUCKET}/db.json${reset}"
}

die() {
  log ERROR "$1"
  exit 1
}

for tool in aws git; do
  command -v "${tool}" >/dev/null 2>&1 || {
    die "Missing required tool: ${tool}."
  }
done

if [[ -z "${WWW_BUCKET:-}" ]]; then
  die "Set WWW_BUCKET in .envrc."
fi

format_duration() {
  local seconds="$1"
  local hours=$((seconds / 3600))
  local minutes=$(((seconds % 3600) / 60))
  local remainder=$((seconds % 60))

  if [[ "${hours}" -gt 0 ]]; then
    printf '%dh %dm %ds' "${hours}" "${minutes}" "${remainder}"
  elif [[ "${minutes}" -gt 0 ]]; then
    printf '%dm %ds' "${minutes}" "${remainder}"
  else
    printf '%ds' "${remainder}"
  fi
}

run_step() {
  local name="$1"
  shift
  local started_at elapsed

  log INFO "Starting ${name}"
  started_at="$(date +%s)"
  if "$@"; then
    elapsed=$(($(date +%s) - started_at))
    log OK "Finished ${name} in $(format_duration "${elapsed}")"
    return 0
  fi

  elapsed=$(($(date +%s) - started_at))
  log ERROR "Failed ${name} after $(format_duration "${elapsed}")"
  return 1
}

pull_radioisotopes() {
  if [[ ! -d "${radioisotopes_dir}/.git" ]]; then
    log ERROR "Expected a git checkout at ${radioisotopes_dir}"
    return 1
  fi

  git -C "${radioisotopes_dir}" pull --ff-only
}

update_once() {
  local started_at elapsed size_bytes had_best_effort_failure

  started_at="$(date +%s)"
  had_best_effort_failure=false
  log INFO "Update cycle started"

  run_step "radioisotopes git pull" pull_radioisotopes || return 1
  if ! run_step "isotope update" \
    "${script_dir}/build-isotopes.sh" "${isotope_args[@]}"; then
    had_best_effort_failure=true
    log WARN "Continuing after isotope update failure; existing isotope metadata will be reused"
  fi
  run_step "Homebrew database update" \
    "${script_dir}/build-db.py" --refresh || return 1
  run_step "combined database build" \
    "${script_dir}/build-combined-json.py" || return 1

  if [[ ! -f "${combined_path}" ]]; then
    log ERROR "Combined database was not created at ${combined_path}"
    return 1
  fi

  size_bytes="$(wc -c <"${combined_path}" | tr -d '[:space:]')"
  run_step "combined database upload to s3://${WWW_BUCKET}/db.json" \
    aws s3 cp "${combined_path}" "s3://${WWW_BUCKET}/db.json" \
      --content-type "application/json" \
      --cache-control "${cache_control}" || return 1

  elapsed=$(($(date +%s) - started_at))
  if [[ "${had_best_effort_failure}" == "true" ]]; then
    log WARN "Update cycle completed with best-effort isotope metadata in $(format_duration "${elapsed}")"
  else
    log OK "Update cycle completed in $(format_duration "${elapsed}")"
  fi
  log OK "Uploaded ${size_bytes} bytes with Cache-Control: ${cache_control}"
  return 0
}

log_header
if update_once; then
  exit 0
fi

status=$?
log ERROR "Update failed with exit status ${status}"
exit "${status}"
