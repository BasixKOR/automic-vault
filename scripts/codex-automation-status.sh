#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
av_db_root="${AV_DB_ROOT:-${repo_root}/../av.db}"
av_www_root="${AV_WWW_ROOT:-${repo_root}/../av.www}"

printf 'workspace: %s\n' "${repo_root}"
printf 'database workspace: %s\n' "${av_db_root}"
printf 'website workspace: %s\n' "${av_www_root}"

printf '\n# av.db automations\n'
"${av_db_root}/scripts/codex-automation-status.sh"

printf '\n# av.www automations\n'
"${av_www_root}/scripts/codex-automation-status.sh"
