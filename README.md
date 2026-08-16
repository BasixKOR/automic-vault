# Automic Vault [![Knock Knock](https://outclaw.dev/badge.svg)](https://outclaw.dev/automic-vault/automic-vault)

> Control how developer credentials are used.

Automic Vault is a macOS secrets manager for developer tools, automations, and
AI agents. It moves supported credentials out of plaintext storage and gives
verified software bounded authority to apply them to specific operations.

## How Automic Vault Differs

Most secrets managers answer: **who may retrieve this named secret?** Once the
secret is returned, their job is done.

Automic Vault answers a different question: **may this verified software use
these credentials for this operation?** Its Authorization Request includes the
Verified Launcher, Tool, Target, command, arguments, working directory, Secret
Names, and selected Secret Value sources. Policy evaluates that complete request
on the Mac where it will run.

That distinction makes one credential useful under different amounts of
authority. With Read Only access, the same GitHub token can produce three
decisions:

```text
gh issue list     → automically authorized
gh issue create   → Approval required
gh auth token     → Secret Disclosure; Approval required
```

The policy is about the requested operation, not merely possession of the
secret. Automic Vault controls the handoff; after a Secret is applied, the
Target controls it.

## Quickstart

Download the [latest release], or install with Homebrew:

```sh
brew install --cask automic-vault/isotopes/automic-vault
open /Applications/Automic\ Vault.app
```

Then audit the machine:

```sh
av scan
```

Automic Vault reports supported Exposures and Hazards and gives each Finding a
specific mitigation. When a Tool supports hardening:

```sh
av harden gh
av doctor gh
```

[latest release]: https://github.com/automic-vault/automic-vault/releases/latest

> [!IMPORTANT]
> Automic Vault is not associated or affiliated with any cryptocurrency or
> “token”.

## What It Protects

The primary adversary is untrusted or compromised code already running with
your normal user privileges: an agent, dependency, plugin, script, or
supply-chain payload.

Automic Vault builds on macOS code signing, the Data Protection Keychain, TCC,
Hardened Runtime, and live process identity. It provides:

- continuous detection for over 100 developer-tool configurations that expose
  credentials or create related hazards;
- Secret Custody outside plaintext dotfiles, environment setup, permissive
  Keychain items, and ambient credential-helper commands;
- Tool-specific Authorization Gates that understand recognized read, write,
  disclosure, elevated, and unknown operations;
- Authorization Policies scoped to each Verified Launcher;
- project-specific values without inventing project prefixes for every Secret
  Name;
- bounded automation through exact, reviewed Blessed Scripts;
- in-memory Temporary Access Grants for eligible agent tasks;
- local Authorization History for allowed and denied requests.

Terminals, IDEs, agents, and projects keep invoking their normal commands.
Automic Vault does not require an agent plugin or a policy file in every
repository.

## Authorization Gates

Hardening adds an Authorization Gate for the Tool. The gate classifies each
request and applies its default Access Level or a rule for the requesting
Verified Launcher.

<img src="./docs/img/authorization-gate-v4.jpg" alt="Automic Vault Authorization Gate" style="width: 589px; height: auto" />

Available levels include:

1. **Approval Required** — every operation needs Approval.
2. **Read Only** — recognized reads are automically authorized.
3. **Read & Update** — Homebrew reads and `brew update` are automically
   authorized.
4. **Local Write** — recognized reads and local-only writes are automically
   authorized where supported.
5. **Write Access** — recognized reads and writes are automically authorized;
   disclosure and elevated application still need Approval.
6. **Full Access** — recognized sensitive operations may also be automically
   authorized; unknown operations still need Approval.

Human Approval is available only while the user session and displays are
active. Open approvals are aborted when that changes. Requests already allowed
by policy may continue, subject to each Secret's **Available While Locked**
setting.

Code signing proves identity and integrity, not good intent. You decide which
Verified Launchers receive authority. If the live identity or runtime posture
cannot be verified, automic authorization fails closed.

## Project Values Without Project-Shaped Names

A Secret Name can have one Global Value and multiple Project Values:

```sh
av save API_TOKEN
av save --project-directory=. API_TOKEN
```

When `av inject` requests `API_TOKEN`, Automic Vault selects the value for the
nearest physical project directory at or above the working directory, then the
Global Value if no Project Value matches. Teams can keep the natural name
`API_TOKEN` across projects instead of maintaining names such as
`ACME_STAGING_API_TOKEN`.

The Project Directory selects a value; it does not grant authority. The same
name-based policy covers all Values of that Secret, and a failure reading the
selected Value does not fall back to another value.

This also composes with encrypted project files. For example, keep dotenvx's
decryption key in Automic Vault instead of `.env.keys`:

```sh
av save --project-directory=. DOTENV_PRIVATE_KEY
av inject +DOTENV_PRIVATE_KEY -- dotenvx run -- npm test
```

dotenvx decrypts the project file only after Automic Vault authorizes applying
its project-selected key to that operation.

## Bounded Automation

### Blessed Scripts

A Blessing binds a canonical script path, exact contents, Script Declaration,
and requested capabilities:

```sh
av bless --endorse-launcher ./scripts/deploy
```

```sh
#!/usr/local/bin/av inject +DEPLOY_TOKEN -- /bin/bash
# --- automic-vault
# capabilities:
#   gh: read-only
#   aws: write
# ---
```

Editing the script or declaration invalidates the Blessing. A Launcher
Endorsement can automically authorize that exact Blessing for a specific
Verified Launcher. Use Blessed Scripts for reviewed, bounded work that exits;
use Tool Authorization Gates for long-running processes.

<img src="./docs/img/blessed-script.png" alt="Automic Vault Blessed Script review" style="width: 589px; height: auto" />

### Temporary Access for Agent Tasks

When an eligible Codex task or Claude Code session makes a write request, the
Approval window can offer **Allow Write Access for 10 Minutes…**. This creates
an in-memory Temporary Access Grant for the exact Verified Launcher,
Tool-specific gate, runtime posture, and current agent task.

<img src="./docs/img/temporary-write-access.png" alt="Automic Vault temporary write access controls" style="width: 589px; height: auto" />

The persistent strip keeps active grants visible and lets you end them early.
They are also revoked when the user session becomes inactive, displays sleep,
an update begins, or Automic Vault stops.

The task identifier narrows the grant but is not identity or a security
boundary; the Verified Launcher remains the identity boundary. Temporary
Access Grants never cover the Direct Secret Gate, Secret mutations, Elevated
Secret Application, Secret Disclosure, or unknown operations.

## Verified Launchers for Unsigned CLIs

Unsigned and arbitrary ad-hoc-signed executables cannot normally be Verified
Launchers. For a regular single-file Mach-O CLI, Automic Vault can create a
Launcher Bundle containing an exact snapshot of that executable:

- the launcher and payload are signed with Hardened Runtime;
- the installed bundle and `/usr/local/bin` command link are root-owned;
- every authorization revalidates the live code identity, generation, payload
  digest, nested signatures, and runtime posture;
- any modification or re-signing hard-denies the request.

Launcher Bundles establish identity and integrity for the packaged code. They
do not establish publisher trust or make the CLI safe. Scripts and
directory-shaped tools are not supported. See [Signed CLI Launchers] for the
complete requirements and update behavior.

[Signed CLI Launchers]: docs/signed-cli-launchers.md

## AWS and Docker Without Ambient Credentials

AWS hardening removes the default long-lived key pair from
`~/.aws/credentials` and installs a native credential helper:

```sh
av harden aws
aws sts get-caller-identity
```

Each invocation registers its arguments, profile, process identity, and config.
Normal commands receive short-lived STS credentials. Operations that require
the original reusable keys show an Elevated Secret Application warning.
Automic Vault installs and verifies AWS's signed CLI under `/opt/av/aws`; other
credential providers outside the supported profile model fail closed.

Docker hardening migrates registry credentials out of ambient helper access:

```sh
av harden docker
docker pull registry.example/acme/image
```

The Secret Gate verifies the live vendor-signed Docker process, ancestry,
arguments, runtime posture, and requested registry. Docker's helper protocol
still returns a usable token to the authorized Docker process; Automic Vault
cannot make a compromised authorized Target keep it confidential.

## Security Boundaries

Automic Vault is deliberately narrower than a system sandbox:

- it controls Secret Application and supported sensitive Tool operations at
  the Local Execution Boundary;
- it does not intercept every process or replace the shell;
- it does not prevent arbitrary local destruction such as `rm -rf`;
- it does not contain root, kernel, or equivalent privileged compromise;
- a Target can leak a Secret after receiving it;
- Project Directories and agent task identifiers narrow selection or scope but
  are not authorization identities;
- local Authorization History is bounded operational history, not a tamperproof
  or audit-complete log.

Use macOS TCC as defense in depth. In particular, avoid giving a general-purpose
terminal or agent harness Full Disk Access or permission to modify other apps
unless its work genuinely requires it.

## Documentation

- [User manual](https://www.automicvault.com/docs/)
- [Domain language](docs/domain-language.md)
- [Architecture](docs/architecture.md)
- [Positioning](docs/positioning.md)
- [Architecture decisions](docs/adr/)
- [Documentation index](docs/index.md)
- [Homebrew tap](https://github.com/automic-vault/homebrew-isotopes)
- [Ephemeral chat](https://outclaw.dev/automic-vault/automic-vault)

Automic Vault is free and open source under Apache-2.0. The optional iPhone
companion can move Approval to a separate device so software on the Mac cannot
approve its own request.
