![promo](./assets/gui-promo.webp)

# Automic Vault

Package manager, secrets manager, and execution control plane for autonomous
agents.

<a href="https://github.com/automic-vault/automic-vault/releases/latest"><img src="./assets/download-button.png" alt="Download Automic Vault .DMG" width="250"></a>


> [!NOTE]
>
> - 20k⭐︎: We’ll add Linux support
> - 50k⭐︎: We’ll add Windows support

> [!IMPORTANT]
>
> Automic Vault is NOT AFFILIATED with any cryptocurrency or token.

[![Coverage Status](https://shieldcn.dev/coveralls/github/automic-vault/automic-vault.svg?variant=outline)](https://coveralls.io/github/automic-vault/automic-vault?branch=main)

&nbsp;


## What is This?

If you got here first then go here before continuing:
[www.automicvault.com](https://www.automicvault.com/).

&nbsp;


## Secure AI Agent Tooling

Automic Vault is a package manager, secrets manager, and approval gate system
for AI agents that run local developer tools. It is built for the moment where
an autonomous coding agent can read files, call command-line tools, and act
with credentials that were originally meant for a human.

Most agent security controls live inside the agent. Automic Vault puts the
boundary beneath the agent: the tools, packages, and secrets it tries to use.
Packages install under controlled roots, secrets stay out of plaintext files,
and sensitive commands can require human approval at execution time.

Use Automic Vault when you need:

- a package manager for AI agents that keeps installed tools harder to modify
- a secrets manager for AI agents that keeps credentials out of model context
- approval gates for commands such as package publishing, token reveal, and
  cloud mutation
- local protection for developer credentials used by GitHub CLI, AWS CLI,
  MCP servers, and other automation tools

Automic Vault is not a replacement for every enterprise secrets platform. It
is the local runtime layer that keeps agent sessions from casually reading or
misusing the credentials and tools already present on a developer machine.

&nbsp;


## Isotopes

Isotope contributor docs now live in [docs/isotopes.md](./docs/isotopes.md).

## Radioisotopes

Radioisotope docs now live in
[data/radioisotopes/README.md](./data/radioisotopes/README.md).

## Next Topes

Keep this list current as new isotopes and radioisotopes land. It should only
include targets that are not already secured.

This list intentionally excludes agent CLIs that are secured by the tools they
run, and excludes dedicated secrets managers that should provide their own
security boundary.

1. `ssh`, `ssh-agent`, and `SSH_AUTH_SOCK`
   - Gate ambient use of unlocked SSH keys by agents and other processes in
     the user's session.
   - Do not replace or delete Apple's `/usr/bin/ssh`; prefer an Automic Vault
     broker, wrappers, and unmanaged-state detection.
2. `brew:kubernetes-cli`
   - Gate production cluster mutation, secret reads, deletes, rollouts, and
     remote execution.
3. `brew:docker`
   - Gate registry credential use, image pushes, privileged containers, and
     host mounts.
4. `git` credential helper and Homebrew `git` integration
   - Gate source-control writes, force pushes, tag mutation, and credential
     use.
   - Do not replace or delete Apple's CLT `git`; control the authentication
     boundary where possible.
5. `brew:azure-cli`
   - Gate cloud resource mutation and Azure identity/token use.
6. `brew:uv`
   - Gate Python package publishing and PyPI token handling.
7. `brew:helm`
   - Gate Kubernetes chart installs, upgrades, deletes, and release mutation.
8. `brew:glab`
   - Gate GitLab source, CI, release, token, and organization administration.
9. `brew:opentofu`
   - Gate infrastructure apply and destroy operations.
10. `brew:ansible`
    - Gate remote fleet mutation and credential use.

&nbsp;
