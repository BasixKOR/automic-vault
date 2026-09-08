# Copying Secrets from v1

Automic Vault 4.6.0 adds `av save --stdin` for exact redirected input. You can
use it to copy selected Values from v1.25.0's legacy login Keychain through the
current app's import Approval.

This is a manual Value copy, not a supported v1 upgrade or rollback procedure.
It does not migrate authorization rules, credential gates, generation/history,
or v1's separate Data Protection Keychain dotenv-key store. Encrypted backup
and recovery remain tracked in [#317](https://github.com/automic-vault/automic-vault/issues/317).

## Before copying

1. Install [4.6.0 or later](https://github.com/automic-vault/automic-vault/releases/latest)
   and open the app. Check `av --version` and `av save --help`. Keep the old
   Keychain and credentials until you have verified their replacements.
2. Identify the Keychain file v1 used. The usual path is
   `~/Library/Keychains/login.keychain-db`; v1 used the default legacy Keychain
   for ordinary `av save` items, under service `com.automicvault.isotope`.
3. Select only the Secret Names you need and check for existing destination
   names. The script creates or updates **Global Values** under those same
   names. An approved import can replace an existing Global Value.

Run as your normal macOS user, without `sudo`. Apple's Swift toolchain/Command
Line Tools must be installed. The script expects the current CLI at
`/usr/local/bin/av`.

## Copy selected Values

Save and review [migrate-av-v1.swift](examples/migrate-av-v1.swift), then run it
with the legacy Keychain path and your Secret Names:

```sh
xcrun swift migrate-av-v1.swift "$HOME/Library/Keychains/login.keychain-db" FOO BILLING_PEM
```

The script reads bytes directly through the Keychain API and pipes them to
`av save --stdin`. It does not put Values in command arguments, environment
variables, output, or plaintext files. It leaves source items and their access
controls intact.

macOS may ask for access to a source item. Choose a one-time **Allow** rather
than granting the Swift interpreter permanent access. Each destination import
still requires the current app's Approval. A failed read, rejected Value, or
denied import stops the script before the next item. Earlier successful imports
remain; this is not a transaction across all selected Secrets.

Values must be nonempty UTF-8 without NUL bytes, at most 1 MiB. The script
preserves the stored bytes, including CRLF and trailing newlines. v1's save
command trimmed surrounding whitespace, so the copy cannot restore bytes that
v1 already removed. Do not substitute `security find-generic-password -w`:
its text/hex formatting and added newline can change a multiline Value.

## Verify before retiring v1 credentials

Start with one selected item. Run its intended consumer through an approved
operation and inspect Authorization History. For a consumer that accepts file
descriptors:

```sh
av inject --mode=fd +BILLING_PEM:3 -- /path/to/consumer
```

The consumer must read FD 3. [FD delivery](direct-secret-access.md#apply-secrets-through-file-descriptors)
requires fresh Approval and bounds each Value by available pipe capacity,
which can be smaller than the 1 MiB save limit. Recreate and review the required
Tool protections and authorization rules separately.

The script was tested with disposable legacy Keychain fixtures and the actual
`av save --stdin` implementation using isolated test storage. Checks covered
multiline PEM text, CRLF, trailing newlines, Unicode, literal `0x…` text, a 1 MiB
Value, rejected imports, invalid input, and unchanged source items. This does
not establish a tested upgrade path for every v1 installation.
