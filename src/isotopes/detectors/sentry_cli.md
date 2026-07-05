# sentry-cli Radioisotope

Sentry CLI can store an auth token in `~/.sentryclirc`.

This radioisotope migrates that global config file into the Automic Vault
keychain and wraps `sentry-cli` so `SENTRY_AUTH_TOKEN` is injected only while
Sentry CLI runs.

The migration removes the `[auth] token` entry from `~/.sentryclirc` and
leaves non-secret config, such as default URL, org, or project values, on
disk. The detector continues to report the config if the auth token reappears.

## Caveats

- We currently migrate the global `~/.sentryclirc` file only.
- Project-local `.sentryclirc` files are not migrated.
- Direct execution of the original binary will not receive the credentials.
