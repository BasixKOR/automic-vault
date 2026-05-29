#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
source "${repo_root}/scripts/cli-style.sh"
cli_style_init "Automic Vault"

source_url="https://automicvault.com/db.json"
output_path="${repo_root}/data/combined.json"
force=false

usage() {
  cat <<'EOF'
Usage: scripts/download-combined-json.sh [--force] [--output PATH]

Download the packaged package database from automicvault.com.

Options:
  --force        Download even when the output file already exists.
  --output PATH  Write the database to PATH.
  --help         Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force)
      force=true
      shift
      ;;
    --output)
      output_path="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      cli_error "Unknown argument: $1"
      usage >&2
      exit 1
      ;;
  esac
done

if [[ "${force}" != "true" && -f "${output_path}" ]]; then
  cli_info "Package database exists: ${output_path}"
  exit 0
fi

mkdir -p "$(dirname "${output_path}")"
temp_path="$(mktemp "${output_path}.XXXXXX")"
cleanup() {
  rm -f "${temp_path}"
}
trap cleanup EXIT

cli_step "Downloading package database"
curl --fail --location --silent --show-error \
  --compressed \
  --output "${temp_path}" \
  "${source_url}"

if [[ ! -s "${temp_path}" ]]; then
  cli_die "Downloaded package database is empty"
fi

mv "${temp_path}" "${output_path}"
trap - EXIT
cli_done "Package database ready"
