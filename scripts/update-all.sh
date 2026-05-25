#!/usr/bin/env bash

set -uo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
daily_hour=3
color_mode="auto"
skip_isotope_builds=false
skip_daily=false
run_website_now=false
run_once=false

usage() {
  cat <<'EOF'
Usage: scripts/update-all [--skip-isotope-builds] [--skip-daily]
                          [--website-now] [--once]
                          [--color auto|always|never] [--no-color]

Run the full publishing cadence:
  - update and upload /db.json at the top of every hour
  - refresh package-page enrichment, regenerate package pages, rebuild search,
    and deploy the static site daily during the 3 AM local-hour slot

Options:
  --skip-isotope-builds             Pass --skip-builds through to update-db.sh.
  --skip-daily                      Disable scheduled daily website publishes.
  --website-now                     Run the website publish immediately on startup.
  --once                            Run the next scheduled hourly slot and exit.
  --color auto|always|never         Control terminal color output.
                                    Defaults to auto.
  --no-color                        Disable terminal color output.
  --help                            Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --db-interval-seconds)
      echo "--db-interval-seconds is no longer supported; database updates run on the hour." >&2
      usage >&2
      exit 1
      ;;
    --daily-interval-seconds)
      echo "--daily-interval-seconds is no longer supported; package-page deploys run at 3 AM." >&2
      usage >&2
      exit 1
      ;;
    --skip-isotope-builds)
      skip_isotope_builds=true
      shift
      ;;
    --skip-daily)
      skip_daily=true
      shift
      ;;
    --website-now)
      run_website_now=true
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

next_hour_epoch() {
  local now remainder
  now="$(date +%s)"
  remainder=$((now % 3600))
  if [[ "${remainder}" -eq 0 ]]; then
    printf '%s\n' "${now}"
  else
    printf '%s\n' $((now + 3600 - remainder))
  fi
}

wait_until_next_hour() {
  local target now sleep_seconds target_label

  target="$(next_hour_epoch)"
  now="$(date +%s)"
  sleep_seconds=$((target - now))
  target_label="$(date -r "${target}" '+%Y-%m-%d %H:%M:%S %Z')"

  if [[ "${sleep_seconds}" -gt 0 ]]; then
    log INFO "Next database update at ${target_label}; sleeping $(format_duration "${sleep_seconds}")"
    sleep "${sleep_seconds}"
  else
    log INFO "Starting scheduled database update for ${target_label}"
  fi

  printf '%s\n' "${target}"
}

is_daily_slot() {
  local scheduled_epoch="$1"
  [[ "$(date -r "${scheduled_epoch}" '+%H')" == "$(printf '%02d' "${daily_hour}")" ]]
}

trap 'log WARN "Stopping update-all"; exit 130' INT TERM

log INFO "${bold}Automic Vault publishing cadence${reset}"
log INFO "Database updates at the top of every hour"
if [[ "${skip_daily}" == "true" ]]; then
  log WARN "Scheduled daily package-page deploy is disabled"
else
  log INFO "Package-page deploy runs daily at 03:00 local time"
fi

last_daily_date=""

if [[ "${run_website_now}" == "true" ]]; then
  log INFO "Immediate website publish requested"
  if run_daily_publish; then
    last_daily_date="$(date '+%Y-%m-%d')"
    log OK "Immediate website publish completed"
  else
    log ERROR "Immediate website publish failed"
    exit 1
  fi
fi

while true; do
  scheduled_epoch="$(wait_until_next_hour)"
  scheduled_date="$(date -r "${scheduled_epoch}" '+%Y-%m-%d')"
  scheduled_label="$(date -r "${scheduled_epoch}" '+%Y-%m-%d %H:%M:%S %Z')"
  cycle_status=0
  if run_database_update; then
    if [[ "${skip_daily}" != "true" ]]; then
      if is_daily_slot "${scheduled_epoch}" && [[ "${last_daily_date}" != "${scheduled_date}" ]]; then
        log INFO "Starting daily package-page publish for ${scheduled_label}"
        if run_daily_publish; then
          last_daily_date="${scheduled_date}"
          log OK "Daily package-page publish completed"
        else
          cycle_status=1
          log ERROR "Daily package-page publish failed; next retry is tomorrow at 03:00"
        fi
      else
        log INFO "Daily package-page deploy is scheduled for 03:00 local time"
      fi
    fi
  else
    cycle_status=1
    log ERROR "Database update failed; retrying at the next hour"
  fi

  if [[ "${run_once}" == "true" ]]; then
    exit "${cycle_status}"
  fi

  sleep 1
done
