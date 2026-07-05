# Oxide CLI Radioisotope

`oxide` stores profile access tokens in
`~/.config/oxide/credentials.toml`. The radioisotope moves that credentials
file into the macOS keychain and recreates it in a temporary HOME only while
`oxide` runs.

## Caveats

- Only the default `~/.config/oxide/credentials.toml` file is migrated.
- Non-secret `config.toml` settings are copied into the temporary HOME.
- Runtime credential edits are not persisted back to keychain.
- Direct execution of the original binary will not receive credentials.
