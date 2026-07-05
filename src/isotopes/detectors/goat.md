# goat Radioisotope

The goat CLI stores AT Protocol account sessions in
`goat/auth-session.json` under the user's XDG state directory. Upstream
documents that this file can contain the app password, access token, and
session token in cleartext.

Automic Vault migrates that session file into the macOS keychain and restores
it under a temporary `XDG_STATE_HOME` only while `goat` runs. This keeps
package-owned local account credentials out of plaintext home-directory state
without changing goat's command-line interface.

## Caveats

- Runtime session refreshes are not persisted back to the keychain.
- Explicit XDG state environment overrides can affect where goat looks.
- Direct execution of the original binary will not receive credentials.
