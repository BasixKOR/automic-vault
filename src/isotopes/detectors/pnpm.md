# pnpm Radioisotope

`pnpm` commonly uses npm auth tokens from `~/.npmrc` when installing private
packages or publishing packages.

This radioisotope migrates a plaintext npm auth token into the Automic Vault
keychain, rewrites `.npmrc` to reference `NODE_AUTH_TOKEN`, and wraps `pnpm` so
that token is injected only while `pnpm` is running.

## Caveats

- We currently support one npm auth token.
- Existing npm config entries are rewritten to reference `NODE_AUTH_TOKEN`.
- This isotope shares the `NODE_AUTH_TOKEN` key with the Node/npm isotope.
