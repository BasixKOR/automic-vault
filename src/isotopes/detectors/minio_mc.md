# MinIO mc Radioisotope

The MinIO Client stores server aliases in `~/.mc/config.json`. Those aliases
can include S3-compatible access keys, secret keys, and session tokens. Automic
Vault migrates each secret-bearing alias into the macOS keychain as an
`MC_HOST_<alias>` environment assignment, removes `secretKey` and `sessionToken`
from the persisted config, and injects those environment variables only while
`mc` runs.

This keeps package-owned local object-storage credentials out of plaintext
home-directory config without changing normal `mc` commands.

## Caveats

- Aliases with names that cannot be exported as `MC_HOST_<alias>` environment
  variables are left for manual handling.
- Runtime alias/config changes are written to the normal config file. The
  detector reports future `secretKey` or `sessionToken` values if they reappear.
- Explicit `--config-dir` arguments can point `mc` at another config file.
- Direct execution of the original binary will not receive credentials.
