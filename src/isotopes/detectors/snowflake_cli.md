# snowflake-cli Radioisotope

Snowflake CLI stores named connections in `config.toml` or `connections.toml`,
and those files can contain plaintext passwords.

This radioisotope migrates supported connection password fields into the
Automic Vault keychain as Snowflake connection environment assignments, then
wraps `snow` so those assignments are exported only while the CLI is running.

The source files are rewritten with blank password values while preserving
non-secret connection settings. If a password reappears in the config files,
the detector will flag it again.

## Caveats

- We currently migrate one default Snowflake config directory at a time.
- Only `password` fields inside supported connection sections are migrated.
- Connection names must map safely to Snowflake's
  `SNOWFLAKE_CONNECTIONS_<NAME>_PASSWORD` environment-variable form.
- `private_key_file_pwd`, unsafe connection names, and passwords outside
  connection sections must be migrated manually.
- Explicit `--config-file` arguments can still point Snowflake CLI at another
  file; those files remain detector-covered rather than wrapper-covered.
- Direct execution of the original binary will not receive credentials.
