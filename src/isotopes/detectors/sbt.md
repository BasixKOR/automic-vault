# sbt Radioisotope

sbt reads global publishing credentials from `~/.sbt/.credentials`. Those
files can contain repository passwords or tokens as plaintext Java properties.

This radioisotope migrates that default credentials file into the macOS
keychain and wraps `sbt` so it receives the credentials through a temporary
file referenced by `SBT_CREDENTIALS`.

## Caveats

- We currently migrate the default `~/.sbt/.credentials` file only.
- Credentials embedded directly in `.sbt` build definitions are not migrated.
- Direct execution of the original binary will not receive credentials.
