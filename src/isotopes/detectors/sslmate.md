# sslmate

The SSLMate CLI stores API credentials in `~/.sslmate` after `sslmate link`.
That file can contain a plaintext `api_key` value used to authenticate SSLMate
API calls.

This radioisotope migrates the default `api_key` into Automic Vault-backed
keychain storage, removes the plaintext key from `~/.sslmate`, and wraps the
installed `sslmate` launcher so the original CLI receives a temporary
`SSLMATE_CONFIG` file for the duration of each command.

Non-secret SSLMate configuration remains in `~/.sslmate`.

## Caveats

- Only the default `~/.sslmate` file is migrated.
- Profile-specific files such as `~/.sslmate-production` and explicit
  `SSLMATE_CONFIG` files are caller-managed contexts and are not migrated.
- Direct execution of `sslmate.av-orig` bypasses Automic Vault injection.
