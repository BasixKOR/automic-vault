# Atlassian CLI Radioisotope

Atlassian CLI stores product authentication profiles in YAML files under
`~/.config/acli`. Authenticated profiles can include API tokens, OAuth access
tokens, refresh tokens, and client secrets.

The radioisotope moves the ACLI config files into the macOS keychain when any
token-bearing config is present and runs `acli` with a temporary `HOME`
containing the injected config set.

## Caveats

- Only the current `~/.config/acli` YAML config layout is migrated.
- Runtime login/logout/profile changes happen in temporary runtime state and
  are not persisted back to the keychain.
- The wrapper runs ACLI with a temporary `HOME`.
- Direct execution of the original binary will not receive credentials.
