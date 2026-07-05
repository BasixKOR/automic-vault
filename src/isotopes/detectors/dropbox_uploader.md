# Dropbox Uploader radioisotope

Dropbox Uploader stores its Dropbox OAuth token in `~/.dropbox_uploader`.
Older configs can also contain OAuth 1 app and token secrets.

This radioisotope migrates those credential values into Automic Vault-backed
keychain storage, removes the plaintext secret assignments from the default
config file, and wraps the installed `dropbox_uploader.sh` script so the
original CLI receives a temporary `-f` config file for each command.

Non-secret config values such as `APPKEY` and `ACCESS_LEVEL` remain in
`~/.dropbox_uploader`.

## Caveats

- Only the default `~/.dropbox_uploader` file is migrated.
- Explicit `-f` config files are caller-managed contexts and are not migrated.
- Direct execution of `dropbox_uploader.sh.av-orig` bypasses Automic Vault
  injection.
