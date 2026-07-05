# OpenHue CLI Radioisotope

`openhue setup` and `openhue config` store the Philips Hue application key in
`config.yaml`. The radioisotope moves that key into the macOS keychain, leaves
the bridge configuration in place, and builds a temporary OpenHue config with
the key injected while `openhue` runs.

The migration checks the active OpenHue config directory:

- `$XDG_CONFIG_HOME/openhue/config.yaml` when `XDG_CONFIG_HOME` is set.
- `~/.openhue/config.yaml` otherwise.

## Caveats

- Only simple YAML `Key: ...` entries are migrated.
- Running OpenHue's setup/config commands can write a new application key back
  to the normal config file; rerun migration after changing the key.
- Direct execution of the original binary will not receive credentials.
