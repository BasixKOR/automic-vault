# Argo CD Radioisotope

The Argo CD CLI stores session tokens in its local config file, usually under
`~/.argocd/config` or `~/.config/argocd/config`.

This radioisotope migrates the first default config file containing auth tokens
into the macOS keychain as `ARGOCD_AUTH_TOKEN`, blanks the persisted
`auth-token` entry, preserves non-secret config such as contexts and servers,
and wraps `argocd` so it receives the token only while it runs.

## Caveats

- We currently migrate the first default config file with exactly one
  `auth-token`.
- Configs with `refresh-token` values or multiple `auth-token` values require
  manual migration.
- Explicit `--config` arguments can select another config file.
- Direct execution of the original binary will not receive credentials.
