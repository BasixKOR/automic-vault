# rclone Radioisotope

rclone stores remote configuration in `rclone.conf`. Many backends persist
tokens, passwords, client secrets, or obscured password values in that file.

This radioisotope migrates the first default `rclone.conf` containing
credential-like values into the macOS keychain and wraps `rclone` so it receives
the config through a temporary `RCLONE_CONFIG` while it runs.

## Caveats

- We currently migrate the first default `rclone.conf` containing credentials.
- Explicit `--config` arguments can override the temporary config.
- Direct execution of the original binary will not receive credentials.
