# Skopeo Radioisotope

`skopeo login` stores container registry credentials in the containers auth
file. On macOS, the package-owned plaintext path is
`~/.config/containers/auth.json`.

The radioisotope moves that auth file into the macOS keychain and rewrites the
auth file to non-secret `credHelpers` entries. The wrapper places a temporary
`docker-credential-av-skopeo` shim on `PATH` and allows Skopeo to fetch
credentials through `av credential-helper skopeo`.

## Caveats

- Only `~/.config/containers/auth.json` is migrated.
- Docker's shared `~/.docker/config.json` and legacy `.dockercfg` are not
  migrated.
- Passing an explicit `--authfile` can bypass the helper-backed auth file.
- Direct execution of the original binary will not receive the helper shim.
