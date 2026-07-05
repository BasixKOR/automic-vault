# transifex-cli Radioisotope

Transifex CLI stores API tokens in `~/.transifexrc`. The file is an INI root
configuration with per-host sections, and each section can include a plaintext
`token` or legacy `password` value.

This radioisotope migrates a single root-config API token into the Automic
Vault keychain as `TX_TOKEN`, with `TX_HOSTNAME` when the config declares a
host. The persisted root config is rewritten with the token blanked while
non-secret settings remain available.

## Caveats

- We currently migrate only the default `~/.transifexrc` root config.
- Root configs with legacy passwords or multiple tokens must be migrated
  manually because they cannot be represented by one `TX_TOKEN` boundary.
- Project-local `.tx/config` files are not migrated.
- Direct execution of the original binary will not receive migrated
  credentials.
