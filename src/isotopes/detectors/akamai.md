# Akamai CLI radioisotope

This radioisotope protects Akamai CLI EdgeGrid credentials that are normally
stored in a plaintext `~/.edgerc` file.

## What it migrates

- `client_token`
- `client_secret`
- `access_token`

The migration converts supported EdgeGrid sections to `AKAMAI_*` environment
assignments stored in the macOS keychain. The on-disk file is rewritten with
blank credential values while preserving non-secret settings.

At runtime the wrapper exports those assignments only while `akamai` runs.

## Caveats

- Only the default `.edgerc` location or `AKAMAI_EDGERC` override is migrated.
- Secret-bearing sections must include `host`, `client_token`,
  `client_secret`, and `access_token`.
- Section names must map safely to Akamai's `AKAMAI_<SECTION>_*`
  environment-variable form.
- Runtime credential changes can write new values to disk; future scans will
  flag them again.
- Direct execution of the original binary will not receive credentials.
