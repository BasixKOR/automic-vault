#!/bin/bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
package="$repo/src/menu-helper"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

swift build \
  --package-path "$package" \
  --configuration debug \
  --product AutomicVaultVarlockPlugin \
  --disable-automatic-resolution
bin_dir=$(swift build \
  --package-path "$package" \
  --configuration debug \
  --show-bin-path)
helper="$bin_dir/AutomicVaultVarlockPlugin"

if [[ "$($helper --protocol-version)" != "1" ]]; then
  echo "error: Varlock helper reported the wrong protocol version" >&2
  exit 1
fi

assert_rejected() {
  local expected=$1
  shift
  local status
  set +e
  "$helper" "$@" >"$work/stdout" 2>"$work/stderr"
  status=$?
  set -e
  if [[ "$status" -ne 1 ]] || ! grep -Fq "$expected" "$work/stderr"; then
    echo "error: Varlock helper accepted invalid input or returned the wrong error" >&2
    sed -n '1,20p' "$work/stderr" >&2
    exit 1
  fi
}

digest=$(printf 'a%.0s' {1..64})
assert_rejected "expected a protocol version" 
assert_rejected "unsupported Varlock protocol version" 2 "$digest" TOKEN
assert_rejected "invalid Varlock schema digest" 1 malformed TOKEN
assert_rejected "invalid Secret Name" 1 "$digest" TOKEN TOKEN
assert_rejected "invalid Secret Name" 1 "$digest" 'BAD-NAME'

too_many=()
for index in {1..65}; do
  too_many+=("TOKEN_$index")
done
assert_rejected "between 1 and 64 Secret Names" 1 "$digest" "${too_many[@]}"

echo "Varlock plugin helper boundary checks passed"
