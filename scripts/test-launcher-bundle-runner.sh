#!/bin/bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
package="$repo/src/menu-helper"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

swift build \
    --package-path "$package" \
    --configuration release \
    --product AutomicVaultLauncher \
    --disable-automatic-resolution

launcher="$work/Test.app/Contents/MacOS/launcher"
payload="$work/Test.app/Contents/Resources/payload"
mkdir -p "$(dirname "$launcher")" "$(dirname "$payload")"
cp "$package/.build/release/AutomicVaultLauncher" "$launcher"
printf 'int main(void) { return 0; }\n' \
    | xcrun clang -arch "$(uname -m)" -x c - -o "$payload"
codesign --force --sign - --options runtime \
    --identifier com.automicvault.launcher-bundle.test.payload "$payload"
payload_hash=$(codesign -dvvv "$payload" 2>&1 | sed -n 's/^CDHash=//p')

AVLB_TEST_HASH="$payload_hash" perl -0777pi -e '
    my $marker = "AVLB_PAYLOAD_CDHASHES:";
    my $hash = $ENV{"AVLB_TEST_HASH"};
    my $offset = 0;
    my $count = 0;
    while (($offset = index($_, $marker, $offset)) >= 0) {
        $offset += length($marker);
        die "Launcher Bundle runner marker was occupied\n"
            unless substr($_, $offset, length($hash)) eq "\0" x length($hash);
        substr($_, $offset, length($hash), $hash);
        $count++;
    }
    die "Launcher Bundle runner marker was not found\n" unless $count;
' "$launcher"
codesign --force --sign - --options runtime \
    --identifier com.automicvault.launcher-bundle.test "$launcher"

"$launcher"
mkdir "$work/bin"
ln -s "$launcher" "$work/bin/test"
"$work/bin/test"
