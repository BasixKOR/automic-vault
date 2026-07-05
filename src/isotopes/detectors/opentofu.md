# opentofu Radioisotope

OpenTofu stores API tokens for remote registries and OpenTofu-compatible
services in `~/.terraform.d/credentials.tfrc.json`.

This radioisotope covers Homebrew core's `opentofu` formula. It migrates that
credentials file into the Automic Vault keychain, installs an OpenTofu
credentials helper shim, and wraps `OpenTofu` so `TF_CLI_CONFIG_FILE` points at
a temporary helper configuration only while OpenTofu is running.

## Caveats

- We currently migrate `credentials.tfrc.json` only.
- Existing `.terraformrc` settings are not merged into the temporary file.
- Direct execution of the original binary will not receive the helper config.
