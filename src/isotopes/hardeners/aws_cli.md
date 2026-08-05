## What It Does

`av harden aws` moves the default AWS access key pair out of
`~/.aws/credentials` and into the macOS login keychain, then installs
`/usr/local/bin/aws` as a one-line Automic Vault launcher for the Homebrew AWS
CLI.

`av doctor aws` recognizes the exact previously released `aws-vault` launcher
as needing rehardening. Modified launchers remain invalid rather than being
treated as an upgrade. `av harden aws` preserves existing Keychain credentials
and gate policy while replacing that launcher.

The launcher registers the exact AWS arguments, selected profile, process ID,
process start time, and a snapshot of the AWS config with the menu app before
replacing itself with `/opt/homebrew/bin/aws`. The AWS CLI receives a minimal
config containing Automic Vault's `credential_process`; that helper only works
as an immediate child of the registered, still-running AWS process.

The menu app implements STS `GetSessionToken` and `AssumeRole` directly. It
caches resulting credentials only for the lifetime of that registered AWS
process. Nothing is written to disk and credentials are not shared between AWS
invocations.

## How It Protects You

The real AWS CLI runs with an empty home, no shared credentials file, disabled
instance metadata, no pager, and a generated config held in an unlinked file
descriptor. Ambient AWS credentials, credential processes, SSO/login state,
web identity, container credentials, plugins, aliases, and pager hooks are not
available inside the credential-bearing process.

The helper verifies all of the following before returning credentials:

- its immediate parent has the registered PID and process start time;
- the parent executable is the interpreter declared by the approved Homebrew
  AWS CLI;
- the live parent arguments exactly match the approved snapshot.

Normal commands receive temporary credentials. AWS does not permit non-MFA
`GetSessionToken` credentials to call IAM or most STS operations, so a base
profile without MFA or a role receives the original long-lived keys for those
operations. The approval window warns prominently and classifies the request
as a secret dump: Trusted Access still prompts, while explicitly selected Full
Access means everything and may auto-approve it.

## Supported Profiles

Automic Vault intentionally supports one narrow profile model:

- the imported `default` keys;
- `region`;
- `mfa_serial`, entered in Automic Vault's own prompt;
- role profiles using `role_arn` and `source_profile`, ultimately rooted at
  `default`.

`mfa_process`, SSO, web identity, `credential_process`, `credential_source`,
independent named static keys, incomplete roles, and source-profile cycles fail
closed with a precise error.

## Caveats

- This assumes `/opt/homebrew/bin/aws`, `/usr/local/bin/av`, and
  `/usr/local/bin/aws`.
- `/usr/local/bin` must precede `/opt/homebrew/bin` in `PATH`; an absolute call
  to the real AWS CLI bypasses the wrapper but cannot access Vault-managed
  credentials.
- `av harden aws` verifies the running app and installed CLI, then requests
  elevation only to atomically replace `/usr/local/bin/aws`.
- The AWS process can use any credential it receives for the lifetime and IAM
  scope of that credential. Automic Vault confines issuance to the approved
  invocation; it cannot harden the upstream AWS CLI process itself.
- End-to-end runtime integrity depends on protecting the Homebrew AWS
  distribution from the desktop user. `av harden brew` is optional, but without
  it the interpreter and source checks can be modified by a same-user attacker.
