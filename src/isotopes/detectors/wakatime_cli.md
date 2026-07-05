# wakatime-cli Radioisotope

WakaTime CLI stores API keys in `$WAKATIME_HOME/.wakatime.cfg`, which defaults
to `~/.wakatime.cfg`. The config can include a default `api_key`,
project-specific API keys, and API URL mappings with embedded keys.

For configs with only the default `[settings] api_key`, this radioisotope
migrates the key into the Automic Vault keychain, replaces the persisted key
with WakaTime's native `api_key_vault_cmd` setting, and wraps `wakatime-cli` so
`av credential-helper wakatime` can read the key only when WakaTime invokes it.

## Caveats

- We currently migrate only the default `~/.wakatime.cfg` file.
- Configs with `project_api_key`, API URL embedded keys, an existing
  `api_key_vault_cmd`, or multiple default API keys require manual migration.
- Project-local `.wakatime` files and imported config files are not migrated.
- Direct execution of the original binary will not receive the credentials.
