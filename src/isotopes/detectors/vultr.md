# vultr Radioisotope

The Vultr CLI reads an API key from `vultr-cli.yaml`, usually in the user's
macOS config directory or at `~/.vultr-cli.yaml`. Automic Vault migrates that
API key into the macOS keychain, removes the `api-key` entry from the
persisted config, and injects `VULTR_API_KEY` only while `vultr-cli` runs.

This keeps package-owned local API keys out of plaintext home-directory config
without changing vultr-cli's command-line interface. Non-secret config entries
remain on disk, and the detector reports the config if `api-key` reappears.

## Caveats

- Only the default macOS config path and legacy `~/.vultr-cli.yaml` location
  are migrated.
- Direct execution of the original binary will not receive credentials.
