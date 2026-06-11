#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
av_www_root="${AV_WWW_ROOT:-${repo_root}/../av.www}"
target="${av_www_root}/scripts/$(basename "$0")"
if [[ ! -x "${target}" ]]; then
  echo "missing av.www package-origin update script: ${target}" >&2
  exit 1
fi
exec "${target}" "$@"

cd "${repo_root}"

log() {
  printf '[%s] %-5s %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$1" "$2" >&2
}

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
  "$@"
  elapsed=$(($(date +%s) - started_at))
  log OK "Finished ${name} in $(format_duration "${elapsed}")"
}

require_publish_env() {
  if [[ -z "${AV_WEB_ORIGIN_SECRET:-}" ]]; then
    log ERROR "Set AV_WEB_ORIGIN_SECRET before running the package-origin publish."
    return 1
  fi

  if [[ -n "${WWW_PKG_ORIGIN_HEADER_VALUE:-}" && "${WWW_PKG_ORIGIN_HEADER_VALUE}" != "${AV_WEB_ORIGIN_SECRET}" ]]; then
    log WARN "WWW_PKG_ORIGIN_HEADER_VALUE is set but does not match AV_WEB_ORIGIN_SECRET."
  fi

  if [[ -n "${WWW_PKG_ORIGIN_HEADER_NAME:-}" && "${WWW_PKG_ORIGIN_HEADER_NAME}" != "${AV_WEB_ORIGIN_HEADER:-X-Automic-Vault-Origin}" ]]; then
    log WARN "WWW_PKG_ORIGIN_HEADER_NAME is set but does not match AV_WEB_ORIGIN_HEADER."
  fi

  if [[ -z "${AV_WEB_CERTBOT_EMAIL:-}" ]]; then
    log WARN "AV_WEB_CERTBOT_EMAIL is unset; Atlas deploy requires an existing TLS cert for ${AV_WEB_ORIGIN_DOMAIN:-av-origin.automicvault.com}."
  fi
}

require_publish_env
run_step "package-origin enrichment refresh" \
  python3 "${script_dir}/generate-pkg-page-enrichment.py" --refresh
run_step "package version freshness generation" \
  python3 "${script_dir}/generate-pkg-version-freshness.py"
run_step "package manager index generation" \
  python3 "${script_dir}/generate-pkg-manager-indexes.py"
run_step "package cross-ecosystem generation" \
  python3 "${script_dir}/generate-pkg-cross-ecosystem.py"
run_step "package graph prepass generation" \
  python3 "${script_dir}/generate-pkg-graph.py"
run_step "package graph curation generation" \
  python3 "${script_dir}/generate-pkg-graph-curation.py"
run_step "package graph generation" \
  python3 "${script_dir}/generate-pkg-graph.py"
run_step "package-origin SQLite generation" \
  python3 "${script_dir}/generate-pkg-sqlite.py"
run_step "Atlas package-origin deploy" \
  "${script_dir}/deploy-pkg-origin.sh" --skip-refresh --skip-sqlite
