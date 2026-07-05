# qwen-code Radioisotope

Qwen Code can store provider API keys in `~/.qwen/settings.json` under the
`env` object. The radioisotope migrates those values into the Automic Vault
keychain as environment-variable assignments, removes them from the persisted
settings file, and exports them only while `qwen` runs.

Non-secret settings remain in `~/.qwen/settings.json`, including provider
configuration that names the API-key `envKey`.

## Caveats

- This isotope currently protects API keys stored in the top-level `env`
  settings object.
- Env keys that cannot be represented as shell environment variable names are
  left for manual handling.
- Deprecated Qwen OAuth `oauth_creds.json` tokens are not migrated.
- Settings changes made while the wrapper runs are written to the normal
  settings file. The detector reports future top-level `env` values if they
  reappear.
- Direct execution of the original binary will not receive credentials.
