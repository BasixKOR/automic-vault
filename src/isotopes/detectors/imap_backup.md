# imap-backup radioisotope

`imap-backup` stores configured IMAP account passwords in
`~/.imap-backup/config.json`. This radioisotope migrates that default config
file into the Automic Vault keychain and supplies it to `imap-backup` through a
temporary `--config` file while the command runs.

The wrapper is intentionally narrow. It only injects the migrated default
config when the user does not pass `--config`, `-c`, or `--erb-configuration`.
Explicit config paths and ERB templates remain under user control.

