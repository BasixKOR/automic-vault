# Vault Radioisotope

The Vault CLI stores its default token in `~/.vault-token` through its internal
token helper. The CLI also honors `VAULT_TOKEN`, which is a clean wrapper
boundary for runtime credential injection.

This radioisotope migrates `~/.vault-token` into the macOS keychain, removes
the plaintext token file, and wraps `vault` so the token is present only while
the command runs.

## Caveats

- We currently migrate the internal token-helper file only.
- Custom external token helpers are not migrated.
- Direct execution of the original binary will not receive credentials.
