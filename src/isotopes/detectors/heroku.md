# Heroku CLI Radioisotope

The Heroku CLI stores login tokens in `.netrc` entries for `api.heroku.com`
and `git.heroku.com`. The radioisotope moves the token into the macOS keychain
as `HEROKU_API_KEY` and injects it while `heroku` runs.

## Caveats

- Only the default `.netrc` path and explicit `NETRC` path are migrated.
- The migration expects the API and Git Heroku entries to use the same token.
- `heroku auth:login` and account switching can write new `.netrc` state; those
  changes are not persisted back to the keychain by the wrapper.
- Direct execution of the original binary will not receive credentials.
