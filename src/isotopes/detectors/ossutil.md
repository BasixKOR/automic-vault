# ossutil radioisotope

ossutil stores Alibaba Cloud OSS credentials in `~/.ossutilconfig` by
default. The config can include `accessKeySecret` and `stsToken` values in
plaintext.

This radioisotope migrates the default config into the Automic Vault keychain
and wraps `ossutil` so it reads a temporary config file only while it is
running.

The source config file is rewritten with blank secret values so long-lived
secrets and temporary tokens are not left in plaintext on disk.

Implemented from the Homebrew popularity scan at formula version 2.3.0.

## Caveats

- We currently migrate the default `~/.ossutilconfig` file only.
- Explicit `-c` or `--config-file` arguments can override the temporary file.
- Settings changes made while `ossutil` runs are written to the temporary
  config and are not persisted back to the keychain.
- Direct execution of the original binary will not receive credentials.
