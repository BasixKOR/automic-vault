# CircleCI CLI

CircleCI CLI stores its API token in `~/.circleci/cli.yml`.

This radioisotope migrates the API token to the keychain, blanks the token in
the local config file, and wraps `circleci` so `CIRCLECI_CLI_TOKEN` is injected
only while the CLI runs.

## Caveats

- If a token is written back into `~/.circleci/cli.yml`, the detector will
  report it again.
- Direct execution of the original binary will not receive credentials.
