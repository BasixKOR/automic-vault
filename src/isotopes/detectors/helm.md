# helm Radioisotope

Helm stores chart repository credentials in `repositories.yaml`. Entries can
contain plaintext usernames, passwords, and client key file references.

This radioisotope migrates the repository config into the Automic Vault
keychain and wraps `helm` so `HELM_REPOSITORY_CONFIG` points at a temporary
copy only while Helm is running.

## Caveats

- We currently migrate Helm repository credentials only.
- OCI registry credentials in Helm's registry config are not migrated yet.
- Explicit `--repository-config` arguments bypass the temporary config.
