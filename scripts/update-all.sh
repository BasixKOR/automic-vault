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
  - refresh av.db Homebrew authority, rebuild, and upload /db.json at the top
    of every hour
  - refresh package-origin enrichment, deploy the Atlas package SQLite origin,
    and deploy the static site once daily at or after the 3 AM local-hour mark

Options:
  --skip-isotope-builds             Pass --skip-builds through to update-db.sh.
  --skip-daily                      Disable scheduled daily website publishes.
  --website-now                     Run the website publish immediately on startup.
  --once                            Run a database update immediately and exit.
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
      echo "--daily-interval-seconds is no longer supported; package-origin publishes run at 3 AM." >&2
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
  local step_name="hourly av.db Homebrew authority and database update"

  if [[ "${skip_isotope_builds}" == "true" ]]; then
    command+=(--skip-isotope-builds)
  fi
  if [[ "${run_once}" == "true" ]]; then
    step_name="av.db Homebrew authority and database update"
  fi
  run_step "${step_name}" "${command[@]}"
}

require_daily_publish_env() {
  local missing=()
  local name

  for name in AV_WEB_ORIGIN_SECRET WWW_PKG_ORIGIN_DOMAIN WWW_PKG_ORIGIN_HEADER_VALUE; do
    if [[ -z "${!name:-}" ]]; then
      missing+=("${name}")
    fi
  done

  if [[ "${#missing[@]}" -gt 0 ]]; then
    for name in "${missing[@]}"; do
      log ERROR "Set ${name} in .envrc before running the website publish."
    done
    return 1
  fi

  if [[ "${WWW_PKG_ORIGIN_HEADER_VALUE}" != "${AV_WEB_ORIGIN_SECRET}" ]]; then
    log ERROR "WWW_PKG_ORIGIN_HEADER_VALUE must match AV_WEB_ORIGIN_SECRET."
    return 1
  fi

  if [[ "${WWW_PKG_ORIGIN_HEADER_NAME:-X-Automic-Vault-Origin}" != "${AV_WEB_ORIGIN_HEADER:-X-Automic-Vault-Origin}" ]]; then
    log ERROR "WWW_PKG_ORIGIN_HEADER_NAME must match AV_WEB_ORIGIN_HEADER."
    return 1
  fi

  if [[ -z "${AV_WEB_CERTBOT_EMAIL:-}" ]]; then
    log WARN "AV_WEB_CERTBOT_EMAIL is unset; Atlas deploy requires an existing TLS cert for ${AV_WEB_ORIGIN_DOMAIN:-av-origin.automicvault.com}."
  fi
}

run_daily_publish() {
  require_daily_publish_env || return 1
  run_step "package-origin enrichment refresh" \
    python3 "${script_dir}/generate-pkg-page-enrichment.py" --refresh || return 1
  run_step "package version freshness generation" \
    python3 "${script_dir}/generate-pkg-version-freshness.py" || return 1
  run_step "package manager index generation" \
    python3 "${script_dir}/generate-pkg-manager-indexes.py" || return 1
  run_step "package cross-ecosystem generation" \
    python3 "${script_dir}/generate-pkg-cross-ecosystem.py" || return 1
  run_step "package graph prepass generation" \
    python3 "${script_dir}/generate-pkg-graph.py" || return 1
  run_step "package graph curation generation" \
    python3 "${script_dir}/generate-pkg-graph-curation.py" || return 1
  run_step "package graph generation" \
    python3 "${script_dir}/generate-pkg-graph.py" || return 1
  run_step "package-origin SQLite generation" \
    python3 "${script_dir}/generate-pkg-sqlite.py" || return 1
  run_step "Atlas package-origin deploy" \
    "${script_dir}/deploy-pkg-origin.sh" --skip-refresh --skip-sqlite || return 1
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

daily_publish_due() {
  local epoch="$1"
  local current_date current_hour

  current_date="$(date -r "${epoch}" '+%Y-%m-%d')"
  current_hour="$(date -r "${epoch}" '+%H')"

  [[ $((10#${current_hour})) -ge "${daily_hour}" && "${last_daily_date}" != "${current_date}" ]]
}

trap 'log WARN "Stopping update-all"; exit 130' INT TERM

log INFO "${bold}Automic Vault publishing cadence${reset}"
if [[ "${run_once}" == "true" ]]; then
  log INFO "Database update will run immediately and exit"
else
  log INFO "av.db Homebrew authority refresh and database upload run at the top of every hour"
  if [[ "${skip_daily}" == "true" ]]; then
    log WARN "Scheduled daily package-origin publish is disabled"
  else
    log INFO "Package-origin publish runs daily at or after 03:00 local time"
  fi
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

if [[ "${run_once}" == "true" ]]; then
  log INFO "Immediate database update requested"
  if run_database_update; then
    exit 0
  fi
  exit 1
fi

while true; do
  wait_until_next_hour >/dev/null
  if run_database_update; then
    if [[ "${skip_daily}" != "true" ]]; then
      daily_check_epoch="$(date +%s)"
      daily_check_date="$(date -r "${daily_check_epoch}" '+%Y-%m-%d')"
      daily_check_label="$(date -r "${daily_check_epoch}" '+%Y-%m-%d %H:%M:%S %Z')"
      if daily_publish_due "${daily_check_epoch}"; then
        log INFO "Starting daily package-origin publish for ${daily_check_label}"
        if run_daily_publish; then
          last_daily_date="${daily_check_date}"
          log OK "Daily package-origin publish completed"
        else
          log ERROR "Daily package-origin publish failed; will retry after the next successful database update"
        fi
      else
        log INFO "Daily package-origin publish is due once per day at or after 03:00 local time"
      fi
    fi
  else
    log ERROR "Database update failed; retrying at the next hour"
    if [[ "${skip_daily}" != "true" ]]; then
      daily_check_epoch="$(date +%s)"
      if daily_publish_due "${daily_check_epoch}"; then
        log WARN "Daily package-origin publish is due; will retry after the next successful database update"
      fi
    fi
  fi

  sleep 1
done
