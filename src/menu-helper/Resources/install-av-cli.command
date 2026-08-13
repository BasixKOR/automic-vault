#!/bin/sh
set -eu

bundle_av="$(CDPATH= cd -- "$(dirname -- "$0")/../MacOS" && pwd)/av"

if [ "${1-}" = "--self-check" ]; then
    [ -x "$bundle_av" ]
    "$bundle_av" __version >/dev/null
    exit
fi

trap 'status=$?; set +x; printf '\''\nPress Return to close this window.'\''; read _; exit "$status"' 0
set -x
/usr/bin/sudo /usr/bin/install "$bundle_av" /usr/local/bin/av
/usr/bin/open -g -b com.automicvault 'automic-vault://cli-installed' || printf '%s\n' 'Installed av, but could not notify Automic Vault.' >&2
