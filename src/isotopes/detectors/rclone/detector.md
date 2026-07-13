# rclone Detector

## Trigger Conditions

- rclone config file contains stored credentials.

## Sensitive Files

- `$RCLONE_CONFIG`
- `$XDG_CONFIG_HOME/rclone/rclone.conf`
- `~/.config/rclone/rclone.conf`
- `~/.rclone.conf`

## Why This is not Yet Hardened

The retired `rclone` hardener moved the detected secret to the macOS Keychain,
then recreated `$RCLONE_CONFIG` inside a temporary directory for each run. We no
longer consider a temporary plaintext file a sufficient security boundary, so
this detector remains report-only.

If a narrow environment-variable or credential-helper interface can cover this
state without writing the secret back to disk, we can reconsider the hardener.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
