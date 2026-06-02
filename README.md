# Automic Vault

A macOS app and CLI for giving AI coding agents useful local tools without
handing them every secret and writable package path on the machine.

<a href="https://github.com/automic-vault/automic-vault/releases/latest"><img src="./assets/download-button.png" alt="Download Automic Vault .DMG" width="250"></a>

> [!IMPORTANT]
> Automic Vault is not affiliated with any cryptocurrency or token.

Homebrew made it normal for developer machines to install the tools they need.
AI agents change the assumption underneath that: the thing running those tools
may not be you.

Automic Vault puts a local boundary under agent work:

- packages install as self-contained packages under controlled roots
- package metadata, install state, updates, and security notes are visible from
  the app and `av`
- secrets are stored in the Automic Vault keychain, not `.env`, shell startup
  files, or model-readable config
- approved secrets can be injected into a specific process when it actually
  needs them
- risky command execution can ask a human before it continues
- local files and isotope detectors can be scanned for plaintext credentials
- `av contain` can run an agent command through a vaulted sandbox and proxy
  toolchain

No magic. Just fewer ambient privileges.

## Install

```sh
$ curl -fsSL https://automicvault.com/install.sh | sh -x
# ^^ downloads the DMG, lets Gatekeeper inspect it, checks TeamIdentifier,
#    copies Automic Vault.app into /Applications, then installs /usr/local/bin/av
```

If `curl | sh` gives you hives, fair:

```sh
$ curl -fsSL https://automicvault.com/install.sh
```

Or download the DMG from [GitHub releases][releases].

## Use It

```sh
$ av --help
# package installs, secret storage/injection, containment, trace, approval gates

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

macOS first.

Linux and Windows are not supported today.

> [!NOTE]
> - 20k stars: we ship Linux
> - 50k stars: we ship Windows

## Hacking

```sh
$ cargo test
$ ./scripts/build-app.sh
```

The native app lives in `src/gui`. The CLI and package/security core live in
`src/lib/rs` and `src/nucleus`.

[releases]: https://github.com/automic-vault/automic-vault/releases/latest
