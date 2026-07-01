# Automic Vault

Secure the tools you `brew install`.

Homebrew made installing developer tools effortless. AI agents changed who is
running them.

Automic Vault adds a local boundary beneath agent sessions: scan for plaintext
credentials, install agent-used packages under controlled roots, keep secrets in
the macOS Keychain, inject them only into approved processes, and ask a human
before commands cross a meaningful risk line.

No magic. Just fewer ambient privileges.

> [!IMPORTANT]
> Automic Vault is not affiliated with any cryptocurrency or token.

[![Coverage Status](https://shieldcn.dev/coveralls/github/automic-vault/automic-vault.svg?variant=outline)](https://coveralls.io/github/automic-vault/automic-vault?branch=main)

&nbsp;


## Why Automic Vault

Developer machines are full of useful ambient authority: package paths, shell
startup files, dotenv files, cloud credentials, GitHub config, MCP servers, and
tools that can publish, delete, deploy, or mutate infrastructure.

Automic Vault makes that authority inspectable and gated:

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
$ av scan
# - finds vulenerable package configurations
# - finds plaintext credentials visible to agents
# - finds insecure macOS configurations

$ av harden aws
# - configures aws to run through aws-vault
# - tells you to remove plaintext AWS keys manually
```

We also have some limited secret management features to help you keep secrets
out of plain text.

```sh
$ printf '%s\n' "$GITHUB_TOKEN" | av save GITHUB_TOKEN
# stores a trimmed secret in the Automic Vault keychain

$ av inject +GITHUB_TOKEN -- gh repo view
# prompts in the app before injecting the secret
# always allow is an option, but requires full paths and checksumming

$ av encrypt ./.env
# ddotenvx compatible encryption

$ av inject --file=.env -- env
# ^^ prompts in the app before allowing `env` to print the secrets
```

Our dotenv story is fairly awesome. [Check it out](./dotenv.md).

&nbsp;


## What Ships

- Menu bar app
  1. Watches your system monitoring for vulnerabilities.
  2. Manages the display of approval gates.
  3. Mediates secret injection.
  4. Aids hardening your system and toolchain.
- `av`, the CLI.

## What This Is Not

No, this does not make agents safe.

No, this is not a replacement for your enterprise vault.

No, this is not a cloud policy engine.

It is a local macOS runtime boundary beneath agent sessions. That is already a
lot, and it is the part we can actually ship.

## Security Guarantees

Under the macOS security model, assuming the machine is not root-compromised,
System Integrity Protection is enabled, the
macOS Keychain is not compromised, and Automic Vault itself is not exploited,
secrets remain protected from ordinary apps, shell tools, malware, and agent
subprocesses. Hardened Runtime blocks normal debugger, injection, and
memory-scraping paths against our signed app, and Keychain only releases secrets
through the authorized Automic Vault code path.

> We also assume that your user is NOT AN ADMINISTRATOR.
>
> This is actually not common. The app will guide you through configuring it.

> We also assume quantum computers are not generally accessible and that whoever
> currently has one poweful enough to break encryption does not have beef with
> you.

&nbsp;


## Platform

macOS: first. Linux & Windows: soon.

> [!NOTE]
> - 20k stars: we ship Linux
> - 50k stars: we ship Windows

## Contributing

```sh
$ ./scripts/sync-isotope-checkouts.sh
$ cargo test
$ ./scripts/run-gui.sh
```

The native app lives in `src/gui`. The CLI and package/security core live in
`src/lib/rs` and `src/nucleus`.

`sync-isotope-checkouts.sh` clones or updates the isotope forks in `../isotopes`
and the radioisotopes checkout in `../radioisotopes`. Override those paths with
`AUTOMIC_VAULT_REPO_CACHE` and `AUTOMIC_VAULT_RADIOISOTOPES_REPO` when needed.


[releases]: https://github.com/automic-vault/automic-vault/releases/latest
[guide-secrets]: https://www.automicvault.com/docs/#guide-secrets
[guide-shebang]: https://www.automicvault.com/docs/#guide-shebang
[guide-dotenv]: https://www.automicvault.com/docs/#guide-dotenv
[guide-containment]: https://www.automicvault.com/docs/#guide-containment
[guide-trace]: https://www.automicvault.com/docs/#guide-trace
