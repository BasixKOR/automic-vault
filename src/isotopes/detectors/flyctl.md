# flyctl Radioisotope

`flyctl` stores its Fly.io API token as `access_token` in `~/.fly/config.yml`.
It also supports `FLY_ACCESS_TOKEN`, which is a clean wrapper boundary for
runtime credential injection.

This radioisotope migrates the saved access token into the macOS keychain,
removes the plaintext token from the config file, and wraps `flyctl` so the
token is present only while the command runs.
If there was no token to migrate, the wrapper leaves `FLY_ACCESS_TOKEN` unset
and lets `flyctl` continue normally.

## Caveats

- We currently migrate `~/.fly/config.yml` only.
- Other non-secret flyctl config entries remain on disk.
- Direct execution of the original binary will not receive credentials.
