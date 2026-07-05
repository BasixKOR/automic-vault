# Grafana CLI radioisotope

Grafana CLI (`grafanactl`) can persist service-account tokens and basic-auth
passwords in its YAML config file. This radioisotope migrates a single
context's token or password into the macOS keychain as Grafana's native
environment variables and blanks the persisted secret while leaving non-secret
context settings on disk.

## Covered credentials

- `$XDG_CONFIG_HOME/grafanactl/config.yaml`
- `~/.config/grafanactl/config.yaml`
- one context containing a `token` value
- one context containing a `password` value, when that context also has `user`

## Caveats

- Configs with multiple secret-bearing contexts must be migrated manually
  because one environment credential set would override context selection.
- Basic-auth password configs must include a user.
- Explicit `--config` arguments can bypass the wrapped config file.
- Environment credentials such as `GRAFANA_TOKEN` still take precedence.
