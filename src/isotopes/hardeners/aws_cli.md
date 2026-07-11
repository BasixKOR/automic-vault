## What It Does

`av harden aws` moves the default AWS access key pair out of
`~/.aws/credentials` and into the macOS login keychain, then installs
`/usr/local/bin/aws` as an Automic Vault wrapper for the Homebrew AWS CLI.

The non-root phase reads the `default` profile from
`${AWS_SHARED_CREDENTIALS_FILE:-$HOME/.aws/credentials}`. If it finds both
`aws_access_key_id` and `aws_secret_access_key`, it stores them as
`AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` under the
`com.automicvault.isotope` keychain service and removes those two plaintext
lines from the credentials file.

The root phase writes `/usr/local/bin/aws`. That wrapper uses `av inject` to
provide the keychain-backed AWS keys only for the command run, creates a
temporary `pass` backend shim for `aws-vault`, and executes
`/opt/homebrew/bin/aws-vault exec "${AWS_PROFILE:-default}" --server --
/opt/homebrew/bin/aws "$@"`.

## How It Protects You

The hardener removes long-lived AWS keys from the ordinary shared credentials
file and keeps them in Keychain instead. The installed wrapper feeds those keys
to `aws-vault` at runtime so the actual AWS CLI runs with short-lived session
credentials rather than reading persistent secrets from disk.

The temporary `pass` shim discards `aws-vault` session-cache writes, so the
wrapper does not leave the generated AWS session material in the temporary
password store after the command exits.

## Caveats

- The import phase only migrates the `default` profile from the shared
  credentials file. The runtime wrapper still uses `${AWS_PROFILE:-default}` and
  follows `source_profile` when preparing the temporary `aws-vault` store.
- This assumes Homebrew paths: `/opt/homebrew/bin/aws-vault`,
  `/opt/homebrew/bin/aws`, and `/usr/local/bin/aws`.
- `/usr/local/bin` must come before the real AWS CLI in `PATH`; otherwise the
  wrapper will not be used.
- The root phase must be run with `sudo av harden aws` because it writes
  `/usr/local/bin/aws`.
- During a command run, the keychain values are injected into the wrapper
  process environment so the temporary `pass` shim can hand them to `aws-vault`.
- The menu helper creates an AWS Secret Gate for the hardened wrapper. Its Read
  Only level uses a conservative allow-list covering `aws s3 ls`, `sts
  get-caller-identity`, `s3api list-*`,
  `s3api head-*`, and service operations named `list-*` or `describe-*`.
  Mutating commands, token-printing commands, manual `av inject`, and unknown
  commands still prompt.
- `aws-vault --server` avoids longer-lived cached credentials but is slower and
  has more runtime overhead.
