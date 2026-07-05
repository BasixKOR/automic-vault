# Composer Radioisotope

Composer stores repository credentials and service tokens in `auth.json`.
Those entries can include HTTP basic passwords, GitHub OAuth tokens, GitLab
tokens, and bearer tokens.

This radioisotope migrates the first default Composer `auth.json` containing
credentials into the macOS keychain and wraps `composer` so it receives those
credentials through `COMPOSER_AUTH` only while it runs.

## Caveats

- We currently migrate the first default `auth.json` containing credentials.
- Project-local `auth.json` files are not migrated.
- Direct execution of the original binary will not receive credentials.
