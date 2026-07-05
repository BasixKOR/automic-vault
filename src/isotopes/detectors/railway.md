# Railway Radioisotope

Railway CLI stores login state in JSON config files under `~/.railway`,
including legacy `token` values and OAuth `accessToken` / `refreshToken`
values. The radioisotope moves those configs into the macOS keychain and runs
`railway` with a temporary home directory containing the injected configs.

## Caveats

- The production, staging, and development config files are migrated:
  `config.json`, `config-staging.json`, and `config-dev.json`.
- Login, logout, token refresh, and project link changes happen in temporary
  runtime state and are not persisted back to the keychain.
- Direct execution of the original binary will not receive credentials.
