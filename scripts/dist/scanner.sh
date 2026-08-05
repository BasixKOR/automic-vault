#!/bin/bash

set -e
umask 077

tmp="$(/usr/bin/mktemp -d /tmp/av-scanner.XXXXXX)"
archive="$tmp/scanner.tgz"
scanner="$tmp/scanner"
profile="$tmp/scanner.sb"
team_id="ZU76A67LGU"
identifier="com.automicvault.scanner"

cleanup() {
  set +x
  /bin/rm -rf "$tmp"
}

trap cleanup EXIT

set -x

# Ignore curl configuration, require HTTPS, and cap the download at 50 MiB.
/usr/bin/curl \
  --disable \
  --fail \
  --silent \
  --show-error \
  --location \
  --proto '=https' \
  --proto-redir '=https' \
  --tlsv1.2 \
  --globoff \
  --max-filesize 52428800 \
  --output "$archive" \
  -- \
  https://www.automicvault.com/scanner.tgz

# Accept exactly one file named scanner, then copy out its contents.
test "$(/usr/bin/tar -tzf "$archive")" = scanner
/usr/bin/tar -xOzf "$archive" scanner >"$scanner"
/bin/chmod 755 "$scanner"

# Require an Apple-issued Developer ID Application signature from our team.
requirement="=anchor apple generic \
and certificate 1[field.1.2.840.113635.100.6.2.6] \
and certificate leaf[field.1.2.840.113635.100.6.1.13] \
and certificate leaf[subject.OU] = \"$team_id\" \
and identifier \"$identifier\""
/usr/bin/codesign --verify --strict -R "$requirement" "$scanner"

# Rust needs process and sysctl reads to start; the scanner itself reads files.
real_scanner="$(cd "$tmp" && pwd -P)/scanner"
/bin/cat >"$profile" <<EOF
(version 1)
(deny default)
(allow file-read*)
(allow process-info*)
(allow sysctl-read)
(allow process-exec (literal "$scanner"))
(allow process-exec (literal "$real_scanner"))
EOF

/usr/bin/sandbox-exec -f "$profile" "$scanner" "$@" </dev/null
