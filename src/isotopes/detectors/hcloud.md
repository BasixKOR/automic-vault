# hcloud Radioisotope

The hcloud CLI stores Hetzner Cloud API tokens in its local TOML config file,
usually `~/.config/hcloud/cli.toml`. Automic Vault migrates a single distinct
token into the macOS keychain as `HCLOUD_TOKEN`, blanks token entries in the
persisted config, and injects the token only while `hcloud` runs.

This keeps package-owned local tokens out of plaintext home-directory config
without changing hcloud's command-line interface.

## Caveats

- Configs with multiple distinct context tokens are left for manual handling.
- Runtime context changes are written to the normal config file. The detector
  reports future token values if they reappear.
- Direct execution of the original binary will not receive credentials.
