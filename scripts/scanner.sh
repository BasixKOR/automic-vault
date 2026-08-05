#!/bin/bash
set -euo pipefail
umask 077

URL="${AUTOMIC_VAULT_SCANNER_URL:-https://www.automicvault.com/scanner.tgz}"
TMP=""

die() {
  echo "scanner: $*" >&2
  exit 1
}

cleanup() {
  if [[ -n "$TMP" && -d "$TMP" ]]; then
    /bin/rm -rf "$TMP"
  fi
}
trap cleanup EXIT

[[ "$(/usr/bin/uname -s)" == Darwin ]] || die "macOS is required"
[[ "$(/usr/bin/uname -m)" == arm64 ]] || die "Apple silicon is required"
for tool in /usr/bin/codesign /usr/bin/curl /usr/bin/sandbox-exec /usr/bin/tar; do
  [[ -x "$tool" ]] || die "$tool is required"
done

TMP="$(/usr/bin/mktemp -d /tmp/av-scanner.XXXXXX)"
ARCHIVE="$TMP/scanner.tgz"
SCANNER="$TMP/scanner"
PROFILE="$TMP/scanner.sb"

/usr/bin/curl --disable --fail --silent --show-error --location \
  --proto '=https' --proto-redir '=https' --tlsv1.2 --globoff \
  --max-filesize 52428800 --output "$ARCHIVE" -- "$URL" || die "download failed"
[[ "$(/usr/bin/tar -tzf "$ARCHIVE")" == scanner ]] || die "invalid archive"
/usr/bin/tar -xOzf "$ARCHIVE" scanner >"$SCANNER" || die "invalid archive"
/bin/chmod 755 "$SCANNER"

REQUIREMENT='=anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] and certificate leaf[field.1.2.840.113635.100.6.1.13] and certificate leaf[subject.OU] = "ZU76A67LGU" and identifier "com.automicvault.scanner"'
/usr/bin/codesign --verify --strict -R "$REQUIREMENT" "$SCANNER" || die "invalid signature"

ESCAPED_SCANNER="${SCANNER//\\/\\\\}"
ESCAPED_SCANNER="${ESCAPED_SCANNER//\"/\\\"}"
REAL_SCANNER="$(cd "$(/usr/bin/dirname "$SCANNER")" && pwd -P)/$(/usr/bin/basename "$SCANNER")"
ESCAPED_REAL_SCANNER="${REAL_SCANNER//\\/\\\\}"
ESCAPED_REAL_SCANNER="${ESCAPED_REAL_SCANNER//\"/\\\"}"
/bin/cat >"$PROFILE" <<EOF
(version 1)
(deny default)
(allow file-read*)
(allow process-info*)
(allow sysctl-read)
(allow process-exec (literal "$ESCAPED_SCANNER"))
(allow process-exec (literal "$ESCAPED_REAL_SCANNER"))
EOF

/usr/bin/sandbox-exec -f "$PROFILE" "$SCANNER" "$@" </dev/null
