# UAA CLI Radioisotope

`uaa` stores access and refresh tokens in `~/.uaa/config.json` by default, or
under `$UAA_HOME/config.json` when that environment variable is set. This
radioisotope moves the default config file into the macOS keychain and exposes
it through a temporary `UAA_HOME` only while `uaa` runs.

## Caveats

- Only the default `~/.uaa/config.json` file is migrated.
- Runtime context changes are not persisted back to keychain.
- Custom `UAA_HOME` locations must be migrated manually.
- Direct execution of the original binary will not receive credentials.
