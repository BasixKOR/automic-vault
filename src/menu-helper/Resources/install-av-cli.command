#!/bin/sh

set -exu

/usr/bin/sudo \
    /usr/bin/install \
        "$(CDPATH= cd -- "$(dirname -- "$0")/../MacOS" && pwd)/av" \
        /usr/local/bin/av

/usr/bin/open \
    -g -b com.automicvault \
    'automic-vault://cli-installed'
