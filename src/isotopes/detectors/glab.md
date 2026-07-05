# GLab Radioisotope

GLab stores GitLab personal access tokens and OAuth refresh tokens in its
`config.yml` file. By default this can live under the legacy
`~/.config/glab-cli` location or the platform XDG config location.

This radioisotope migrates a single host personal access token into the macOS
keychain as `GITLAB_TOKEN` and `GITLAB_HOST`, rewrites the persisted config
with the token blanked, and wraps `glab` so only those environment variables
are injected while it runs.

## Caveats

- We currently migrate the first default global `config.yml` containing a
  token.
- Configs with OAuth refresh tokens or multiple host tokens must be migrated
  manually because they cannot be represented by one token environment pair.
- Local `.git/glab-cli/config.yml` files are not migrated.
- Direct execution of the original binary will not receive migrated
  credentials.
