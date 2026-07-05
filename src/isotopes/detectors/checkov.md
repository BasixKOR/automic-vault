# Checkov Radioisotope

Checkov reads a Bridgecrew/Prisma API key from `~/.bridgecrew/credentials`
when `BC_API_KEY` or `--bc-api-key` is not supplied. The radioisotope moves
that plaintext credential into the macOS keychain and injects it as
`BC_API_KEY` while `checkov` runs.

## Caveats

- Only `~/.bridgecrew/credentials` is migrated.
- Runtime changes to the Bridgecrew credential file are not persisted back to
  the keychain.
- An explicit `--bc-api-key` argument supplied by the user takes precedence in
  Checkov's normal argument parsing.
- Direct execution of the original binary will not receive credentials.
