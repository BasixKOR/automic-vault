# snyk

Snyk stores local CLI configuration in Configstore at
`$XDG_CONFIG_HOME/configstore/snyk.json`, normally
`~/.config/configstore/snyk.json`. That file can contain the Snyk API token
and OCI registry credentials.

This radioisotope migrates API and OAuth tokens into the keychain as
`SNYK_TOKEN` / `SNYK_OAUTH_TOKEN`, rewrites the persisted configstore JSON with
those token values blanked, and preserves non-secret settings on disk.

## Caveats

- Configs with OCI registry passwords or client secrets must be migrated
  manually because those secret shapes are not covered by `SNYK_TOKEN`.
- `SNYK_TOKEN`, `SNYK_OAUTH_TOKEN`, and `SNYK_CFG_*` environment variables can
  override migrated config.
- Direct execution of the original binary will not receive migrated
  credentials.
