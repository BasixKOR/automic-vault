# Bitwarden CLI Radioisotope

Bitwarden CLI stores its app state in `data.json`. In the CLI build,
`supportsSecureStorage()` returns false, so access and refresh token state can
fall back to that plaintext lowdb file.

The radioisotope moves token-bearing `data.json` contents into the macOS
keychain and runs `bw` with a temporary `BITWARDENCLI_APPDATA_DIR`.

## Caveats

- Only the default app data path and explicit `BITWARDENCLI_APPDATA_DIR` are
  migrated.
- Login, logout, sync, and token refreshes write to temporary runtime state and
  are not persisted back to the keychain.
- Direct execution of the original binary will not receive credentials.
