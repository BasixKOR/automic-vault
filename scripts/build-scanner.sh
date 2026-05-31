#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
source "${repo_root}/scripts/cli-style.sh"
cli_style_init "Automic Vault"

output_path="${repo_root}/target/scanner.gz"

usage() {
  cat <<'EOF'
Usage: scripts/build-scanner.sh [--output PATH]

Build the minimal isotope-only scanner binary and gzip it.

Options:
  --output PATH  Write the gzip-compressed scanner to PATH.
  --help         Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
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

mkdir -p "$(dirname "${output_path}")"
output_dir="$(cd "$(dirname "${output_path}")" && pwd)"
output_path="${output_dir}/$(basename "${output_path}")"
scanner_path="${repo_root}/target/release/scanner"

cli_title "Build Automic Vault scanner"
cli_step "Building size-optimized scanner"

rustflags="${RUSTFLAGS:-}"
if [[ "$(uname -s)" == "Darwin" ]]; then
  rustflags="${rustflags} -C link-arg=-Wl,-dead_strip"
fi

env \
  CARGO_PROFILE_RELEASE_OPT_LEVEL=z \
  CARGO_PROFILE_RELEASE_LTO=fat \
  CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
  CARGO_PROFILE_RELEASE_STRIP=symbols \
  CARGO_PROFILE_RELEASE_PANIC=abort \
  RUSTFLAGS="${rustflags}" \
  cargo build \
    --release \
    --bin scanner \
    --manifest-path "${repo_root}/Cargo.toml" \
    >&2

if [[ ! -x "${scanner_path}" ]]; then
  cli_die "Expected built scanner at ${scanner_path}"
fi

cli_step "Compressing scanner"
tmp_output="$(mktemp "${output_path}.XXXXXX")"
gzip -9 -n -c "${scanner_path}" >"${tmp_output}"
mv "${tmp_output}" "${output_path}"
chmod 644 "${output_path}"

scanner_bytes="$(wc -c <"${scanner_path}" | tr -d '[:space:]')"
gzip_bytes="$(wc -c <"${output_path}" | tr -d '[:space:]')"
cli_info "Binary: ${scanner_bytes} bytes"
cli_info "Gzip: ${gzip_bytes} bytes"
cli_done "Scanner ready"
printf '%s\n' "${output_path}"
