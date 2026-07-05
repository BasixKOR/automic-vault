# Graphite CLI Radioisotope

Graphite CLI stores authentication in its XDG config directory, usually
`~/.config/graphite/auth`. It can also store profile auth tokens in
`~/.config/graphite/user_config`.

The radioisotope moves the token-bearing config into the macOS keychain and
runs `gt` with a temporary `XDG_CONFIG_HOME` containing the injected Graphite
config files.

## Caveats

- Only the current XDG config layout is migrated.
- `auth`, `user_config`, and `aliases` are injected when available, but runtime
  auth/config/alias changes are not persisted back to the keychain.
- If `user_config` contains auth tokens, the original file is replaced with an
  empty JSON object after migration.
- Direct execution of the original binary will not receive credentials.
