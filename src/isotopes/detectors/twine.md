# isotope:twine

Twine reads Python package index credentials from `~/.pypirc` by default.
That file can contain plaintext repository passwords or API tokens.

This radioisotope migrates a single default `.pypirc` repository credential into
Automic Vault storage as Twine's native `TWINE_*` environment variables,
sanitizes local password-bearing entries, and wraps `twine` so only those
environment variables are injected while Twine runs.

## Migrated Data

- one `~/.pypirc` repository section containing `password = ...`
- one repository URL in `~/.pypirc` that embeds userinfo

## Caveats

- Only the default `~/.pypirc` path is migrated.
- `.pypirc` files with credentials for multiple repositories must be migrated
  manually.
- Custom repository sections need a repository URL and, unless they target
  PyPI/TestPyPI, a username.
- Environment variables such as `TWINE_PASSWORD` are not migrated.
- Direct execution of the original binary will not receive credentials.
