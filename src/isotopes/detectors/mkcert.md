# mkcert Radioisotope

`mkcert` creates a local certificate authority and stores its private key as
`rootCA-key.pem` in the user CAROOT directory.

This radioisotope migrates `rootCA-key.pem` into the Automic Vault keychain and
wraps `mkcert` so the key is materialized in a temporary CAROOT only while
`mkcert` is running.

## Caveats

- We currently migrate the default CAROOT, or the CAROOT set during migration.
- The public `rootCA.pem` file remains on disk.
- Existing shells that execute the original binary directly will not receive
  the root CA key.
