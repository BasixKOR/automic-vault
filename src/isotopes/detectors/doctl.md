# doctl Radioisotope

`doctl auth init` stores DigitalOcean access tokens in `config.yaml`. The
radioisotope moves supported default-context tokens into the macOS keychain and
injects `DIGITALOCEAN_ACCESS_TOKEN` while `doctl` runs.

The source config is rewritten with a blank `access-token` while preserving
non-secret settings. If a token reappears in the config file, the detector will
flag it again.

## Caveats

- Only default-context top-level `access-token` values are migrated.
- Named `auth-contexts` tokens and non-default contexts must be migrated
  manually because `DIGITALOCEAN_ACCESS_TOKEN` only covers the default context.
- `doctl auth init` can write a new token back to the config file; the detector
  will flag that token on future scans.
- Direct execution of the original binary will not receive credentials.
