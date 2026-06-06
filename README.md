# Automic Vault

Secure the tools you `brew install`.

Automic Vault sits discreetly on top of Homebrew, hardening it. Packages store
their secrets securely. Human approval is required for
exfiltration of those secrets & risky commands.

> [!IMPORTANT]
> Automic Vault is not affiliated with any cryptocurrency or token.

[![Coverage Status](https://shieldcn.dev/coveralls/github/automic-vault/automic-vault.svg?variant=outline)](https://coveralls.io/github/automic-vault/automic-vault?branch=main)

&nbsp;


## Why Automic Vault

Homebrew taught developer machines to install whatever tools the job needs.
AI agents change the deal: the thing running those tools may not be you.

Automic Vault adds a local boundary for agent work:

- packages install as self-contained packages under controlled roots
- the app and `av` show package metadata, install state, updates, and security
  notes
- secrets are stored in the Automic Vault keychain, not `.env`, shell startup
  files, or model-readable config
- approved secrets are injected only into the process that needs them
- risky command execution can ask a human before it continues
- `av` can scan local files and isotope detectors for plaintext credentials
- `av contain` can run an agent command through a vaulted sandbox and proxy
  toolchain

No magic. Just fewer ambient privileges.

&nbsp;


## Install It

```sh
curl -fsSL https://automicvault.com/install.sh | sh && av open
# ^^ downloads and mounts the DMG read-only
#    lets Gatekeeper inspect the app
#    verifies its signature and TeamIdentifier
#    copies Automic Vault.app into /Applications
#    sudo installs /usr/local/bin/av
```

If `curl | sh` gives you hives, fair. You can just download the DMG from
[GitHub releases][releases].

## Use It

```sh
$ av --help
# package installs, secret storage/injection, containment, trace, approval gates

$ av open
# opens Automic Vault.app

$ av info jq
# source, version, install state, dependencies, homepage, license

$ av install jq
# installs a self-contained package

$ av scan --path .
# finds plaintext credentials visible to agents

$ printf '%s\n' "$GITHUB_TOKEN" | av save GITHUB_TOKEN
# stores a trimmed secret in the Automic Vault keychain

$ av inject +GITHUB_TOKEN /opt/homebrew/bin/gh repo view
# asks Automic Vault to approve injecting that key into that process

$ av contain codex
# runs codex with generated stubs that request approved host execution
```

For the rest:

```sh
$ av <subcommand> --help
```

## Stop Exporting Secrets From Your Shell

Do not put long-lived credentials in `.zshrc`, `.zprofile`, `.zshenv`,
`.bashrc`, `.bash_profile`, `.profile`, `.envrc`, or project `.env` files that
agents can read. Those files are convenient, but they make every shell and
every local automation process an ambient credential carrier.

Save the value once:

```sh
$ printf '%s\n' "$OPENAI_API_KEY" | av save OPENAI_API_KEY
$ unset OPENAI_API_KEY
```

Then inject it only into the tool that needs it:

```sh
$ av inject +OPENAI_API_KEY /opt/homebrew/bin/opencode run
$ av inject +GITHUB_TOKEN /opt/homebrew/bin/gh repo view automic-vault/automic-vault
$ av inject +AWS_ACCESS_KEY_ID +AWS_SECRET_ACCESS_KEY /opt/homebrew/bin/aws sts get-caller-identity
```

`+KEY` names a value stored in the Automic Vault keychain. The target must be an
absolute executable or script path. By default, `av inject` refuses to overwrite
an environment variable that is already set; use `--replace-existing-env` when
you deliberately want the keychain value to win. Wrapper scripts can use
`--allow-missing-keys` when a tool has optional credentials.

For repeatable workflows, make the script itself use `av inject` as its
interpreter:

```sh
#!/usr/local/bin/av inject +OPENAI_API_KEY /bin/sh
set -eu
exec /opt/homebrew/bin/opencode run "$@"
```

or for a tool with optional credentials:

```sh
#!/usr/local/bin/av inject --allow-missing-keys +GITHUB_TOKEN /bin/sh
set -eu
exec /opt/homebrew/bin/gh "$@"
```

Make the script executable and run it normally. The kernel starts `av`, `av`
asks Automic Vault to approve the named key for that script and interpreter,
then `/bin/sh` receives the key only for that execution.

This is the boundary Automic Vault is trying to create:

- the secret value lives in the macOS Keychain, not in a shell startup file,
  project file, prompt, transcript, or agent-readable config
- approval shows the key names, target executable, arguments, current
  directory, parent process, and script context before injection
- always-allow decisions are scoped to the executable and, for user-controlled
  scripts, the script path and SHA-256 so changed scripts ask again
- `av inject` disables core dumps, validates the target path and parent
  directories, and on macOS verifies the opened executable still matches the
  path immediately before `execve`
- the value is placed in the child process environment only after approval; the
  model does not get the raw value unless the tool itself prints or leaks it

That is not a substitute for central enterprise secret management. It is a
local runtime handoff that keeps agent-readable files clean while still letting
real command-line tools authenticate when they run.

## What Ships

- `Automic Vault.app`: the package console, package dossiers, recommendations,
  update UI, and approval prompts
- `av`: the CLI for package, secret, approval, containment, trace, and local
  daemon workflows
- `nuke-helper`: the privileged helper for operations that need it
- isotope and approval-gate metadata for package-specific security behavior

## What This Is Not

No, this does not make agents safe.

No, this is not a replacement for your enterprise vault.

No, this is not a cloud policy engine.

It is a local macOS runtime boundary beneath agent sessions. That is already a
lot, and it is the part we can actually ship.

## Platform

macOS: first. Linux & Windows: soon.

> [!NOTE]
> - 20k stars: we ship Linux
> - 50k stars: we ship Windows

## Hacking

```sh
$ cargo test
$ ./scripts/run-gui.sh
```

The native app lives in `src/gui`. The CLI and package/security core live in
`src/lib/rs` and `src/nucleus`.

## Dependencies

- `ecies` is used by `av dotenv` to read and write dotenvx-compatible
  `encrypted:` values using the same secp256k1 ECIES scheme as dotenvx.
- `base64` is used by `av dotenv` for dotenvx-compatible encrypted value
  encoding.
- `rusqlite` with bundled SQLite is used by `av-web`, the private Atlas package
  origin. It serves the locally generated `pkg.sqlite` artifact without adding a
  system SQLite shared-library dependency on Atlas.

[releases]: https://github.com/automic-vault/automic-vault/releases/latest
