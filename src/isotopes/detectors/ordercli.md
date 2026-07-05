# ordercli Radioisotope

`ordercli` stores provider session state in its JSON config, including Foodora
access/refresh tokens, client secrets, MFA tokens, browser cookies, and Glovo
access tokens. The radioisotope moves a token-bearing config into the macOS
keychain and runs `ordercli` with a temporary home directory containing that
config.

## Caveats

- Only the default config path and legacy `foodcli` / `foodoracli` paths are
  migrated.
- Login, logout, session refresh, and config writes happen in temporary runtime
  state and are not persisted back to the keychain.
- Direct execution of the original binary will not receive credentials.
