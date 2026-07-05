# uv Radioisotope

`uv auth login` stores HTTP package index credentials in a plaintext
`credentials.toml` file by default.

This radioisotope migrates that credentials file into the Automic Vault
keychain and wraps `uv` so the credentials are reconstructed in a temporary
`UV_CREDENTIALS_DIR` only while `uv` is running. If `uv` changes the temporary
credentials file, the wrapper saves the updated file back into Automic Vault
before removing the temporary directory.

## Caveats

- We currently migrate the default uv credentials file only.
- `UV_CREDENTIALS_DIR` overrides are detected but must be migrated manually.
- `uv`'s `native-auth` preview stores credentials outside Automic Vault, so the
  wrapper rejects invocations that enable it.
