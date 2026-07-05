# s3cmd Radioisotope

s3cmd stores S3 access credentials in `~/.s3cfg` by default.

This radioisotope migrates access credentials into the Automic Vault keychain
as AWS-compatible environment variables and wraps `s3cmd` so only those
environment variables are injected while it is running.

The source config file is rewritten with blank access/session keys and a
non-secret `gpg_passphrase = $S3CMD_GPG_PASSPHRASE` reference, so secrets are
not left in plaintext on disk while non-secret settings remain available.

## Caveats

- We currently migrate the default `~/.s3cfg` file only.
- `access_key` and `secret_key` must both be present when either one is
  migrated.
- Explicit `--config` or `-c` arguments can point `s3cmd` at another config
  file whose own non-empty credential fields may take precedence over env vars.
- Direct execution of the original binary will not receive credentials.
