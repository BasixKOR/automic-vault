#!/usr/bin/env bash

set -uo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
db_interval_seconds=3600
daily_interval_seconds=86400
color_mode="auto"
skip_isotope_builds=false
skip_daily=false
run_once=false

usage() {
  cat <<'EOF'
Usage: scripts/update-all [--db-interval-seconds SECONDS]
                          [--daily-interval-seconds SECONDS]
                          [--skip-isotope-builds] [--skip-daily]
                          [--once]
                          [--color auto|always|never] [--no-color]

Run the full publishing cadence:
  - update and upload /db.json every hour
  - refresh package-page enrichment, regenerate package pages, rebuild search,
    and deploy the static site once per day

Options:
  --db-interval-seconds SECONDS     Database update cadence. Defaults to 3600.
  --daily-interval-seconds SECONDS  Package-page deploy cadence. Defaults to 86400.
  --skip-isotope-builds             Pass --skip-builds through to update-db.sh.
  --skip-daily                      Only run the hourly database cadence.
  --once                            Run one scheduler cycle and exit.
  --color auto|always|never         Control terminal color output.
                                    Defaults to auto.
  --no-color                        Disable terminal color output.
  --help                            Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --db-interval-seconds)
      if [[ $# -lt 2 ]]; then
        echo "--db-interval-seconds requires a value" >&2
        exit 1
      fi
      db_interval_seconds="$2"
      shift 2
      ;;
    --daily-interval-seconds)
      if [[ $# -lt 2 ]]; then
        echo "--daily-interval-seconds requires a value" >&2
        exit 1
      fi
      daily_interval_seconds="$2"
      shift 2
      ;;
    --skip-isotope-builds)
      skip_isotope_builds=true
      shift
      ;;
    --skip-daily)
      skip_daily=true
      shift
      ;;
    --once)
      run_once=true
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

for interval_name in db_interval_seconds daily_interval_seconds; do
  interval_value="${!interval_name}"
  if ! [[ "${interval_value}" =~ ^[0-9]+$ ]] || [[ "${interval_value}" -eq 0 ]]; then
    echo "--${interval_name//_/-} must be a positive integer" >&2
    exit 1
  fi
done

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

format_duration() {
  local seconds="$1"
  local days=$((seconds / 86400))
  local hours=$(((seconds % 86400) / 3600))
  local minutes=$(((seconds % 3600) / 60))
  local remainder=$((seconds % 60))

  if [[ "${days}" -gt 0 ]]; then
    printf '%dd %dh %dm %ds' "${days}" "${hours}" "${minutes}" "${remainder}"
  elif [[ "${hours}" -gt 0 ]]; then
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

run_database_update() {
  local command=("${script_dir}/update-db.sh" "--color" "${color_mode}")
  if [[ "${skip_isotope_builds}" == "true" ]]; then
    command+=(--skip-isotope-builds)
  fi
  run_step "hourly database update" "${command[@]}"
}

run_daily_publish() {
  run_step "package-page enrichment refresh" \
    python3 "${script_dir}/generate-pkg-page-enrichment.py" --refresh || return 1
  run_step "package-page generation" \
    python3 "${script_dir}/generate-pkg-pages.py" || return 1
  run_step "Pagefind search index generation" \
    python3 "${script_dir}/generate-search-index.py" || return 1
  run_step "static site deploy" \
    "${script_dir}/deploy-www.sh" || return 1
}

sleep_until_next_cycle() {
  local cycle_started_at="$1"
  local elapsed sleep_seconds

  elapsed=$(($(date +%s) - cycle_started_at))
  if [[ "${elapsed}" -lt "${db_interval_seconds}" ]]; then
    sleep_seconds=$((db_interval_seconds - elapsed))
    log INFO "Sleeping $(format_duration "${sleep_seconds}") until the next database update"
    sleep "${sleep_seconds}"
  else
    log WARN "Cycle took $(format_duration "${elapsed}"); starting the next database update immediately"
  fi
}

trap 'log WARN "Stopping update-all"; exit 130' INT TERM

log INFO "${bold}Automic Vault publishing cadence${reset}"
log INFO "Database updates every $(format_duration "${db_interval_seconds}")"
if [[ "${skip_daily}" == "true" ]]; then
  log WARN "Daily package-page deploy is disabled"
else
  log INFO "Package-page deploys every $(format_duration "${daily_interval_seconds}")"
fi

last_daily_at=0

while true; do
  cycle_started_at="$(date +%s)"
  cycle_status=0
  if run_database_update; then
    if [[ "${skip_daily}" != "true" ]]; then
      now="$(date +%s)"
      if [[ $((now - last_daily_at)) -ge "${daily_interval_seconds}" ]]; then
        if run_daily_publish; then
          last_daily_at="$(date +%s)"
          log OK "Daily package-page publish completed"
        else
          cycle_status=1
          log ERROR "Daily package-page publish failed; retrying after the next database update"
        fi
      else
        next_daily_in=$((daily_interval_seconds - (now - last_daily_at)))
        log INFO "Next package-page deploy in $(format_duration "${next_daily_in}")"
      fi
    fi
  else
    cycle_status=1
    log ERROR "Database update failed; retrying after the next interval"
  fi

  if [[ "${run_once}" == "true" ]]; then
    exit "${cycle_status}"
  fi

  sleep_until_next_cycle "${cycle_started_at}"
done
