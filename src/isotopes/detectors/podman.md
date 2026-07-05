# Podman Radioisotope

Podman stores registry login credentials in a containers `auth.json` file.
Those entries can include base64 `auth` values or identity tokens for container
registries.

This radioisotope migrates the first default user auth file it finds into the
macOS keychain and rewrites the auth file to non-secret `credHelpers` entries.
The wrapper places a temporary `docker-credential-av-podman` shim on `PATH` and
allows Podman to fetch credentials through `av credential-helper podman`.

## Caveats

- We currently migrate default user auth files only.
- Explicit `--authfile` arguments can bypass the helper-backed auth file.
- Direct execution of the original binary will not receive the helper shim.
