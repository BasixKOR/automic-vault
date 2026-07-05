# Alibaba Cloud CLI radioisotope

This radioisotope protects Alibaba Cloud CLI credentials that are normally
stored in plaintext JSON at `~/.aliyun/config.json`.

## What it migrates

It stores the full config JSON in the macOS keychain when any configured
profile contains inline credential material such as:

- `access_key_secret`
- `sts_token`
- `private_key`
- `access_token`
- `oauth_access_token`
- `oauth_refresh_token`

The migration rewrites the local config to a valid JSON document with those
secret fields blanked. At runtime the wrapper restores the original config
under a temporary `HOME`.

## Caveats

- Only the default `~/.aliyun/config.json` location is migrated.
- Explicit `--config-path` arguments are not intercepted.
- Runtime config changes are not persisted back to keychain.
- Direct execution of the original binary will not receive credentials.
