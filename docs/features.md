# Automic Vault Features

Automic Vault is a local security runtime for autonomous coding agents. It
protects the tools, credentials, and command boundaries an agent touches on a
developer machine.

## Core Feature Areas

| Feature | What it does | User value |
| --- | --- | --- |
| Nucleus package manager | Installs Homebrew formulae, casks, npm packages, PyPI packages, and Automic Vault isotopes under controlled roots. | Gives agents useful tools without letting package installs sprawl across the machine. |
| Package search and dossier | Shows package metadata, install status, versions, dependencies, executables, install destination, and update state. | Lets users inspect what a package is and where it will run before acting. |
| External package surfaces | Embeds package homepages and related package pages in the app. | Keeps package context visible while deciding whether to install, update, or trust a tool. |
| Updates and outdated checks | Lists installed packages with newer versions and reinstalls packages with updates available. | Keeps agent toolchains current without hiding what changed. |
| Isotopes | Uses secured forks of open source tools with explicit approval gates added at risky execution points. | Moves security boundaries into the tools agents actually run. |
| Radioisotopes | Uses package metadata and detectors to identify risky packages before a full isotope exists. | Gives earlier warnings and remediation paths for known tool risks. |
| Approval gates | Blocks sensitive actions until a human approves or denies them. | Prevents agents from silently publishing, deleting, mutating infrastructure, or exposing secrets. |
| Containment | Runs agents with command approval gates around their tool execution. | Creates a practical review layer beneath the agent session. |
| Secrets keychain | Stores secrets in the Automic Vault keychain and injects them only into approved processes. | Keeps credentials out of plaintext files, shell history, and model context. |
| Credential helper adapter | Provides an approved credential-helper path for tools that need credentials at runtime. | Lets existing tools request credentials through a controlled boundary. |
| Secret scanner | Scans isotope detectors and likely local secret files for plaintext credentials. | Finds credentials that are already visible to agents before they are used or leaked. |
| Static trace | Asks a local agent to explain likely file-changing steps in a shell one-liner without running it. | Helps users understand command risk before execution. |
| Local protocol daemon | Starts a local read-only protocol daemon for app and tool coordination. | Keeps local status and package information available without making package metadata remotely configurable. |
| macOS app | Provides a native AppKit interface for browsing packages, reviewing security state, and approving sensitive actions. | Makes agent security decisions visible and interruptible without requiring terminal fluency. |
| Menu bar status | Shows Nucleus/app status and update state from a lightweight macOS menu surface. | Gives quick operational feedback without opening the full app. |

## Current Product Shape

Automic Vault has three primary user surfaces:

1. `av` CLI for package, secret, approval, containment, trace, and daemon
   workflows.
2. Automic Vault.app for package browsing, package dossiers, security status,
   external package context, and approval prompts.
3. Repository-maintained isotope and approval metadata for package-specific
   security behavior.

## Package Detail Experience

The package detail experience should remain centered on a simple flow:

1. Search or select a package.
2. Read the dossier: version, source, install state, dependencies, executable
   paths, install destination, and security status.
3. Review the package homepage by default.
4. Switch to related pages when available: release notes, GitHub repository,
   README, docs, registry page, or changelog.
5. Install, update, migrate, make default, or remediate only after the package
   context is visible.

Homepage visibility is a product feature, not incidental web chrome. Related
pages should be easy to reach, but they should not replace the homepage by
surprise.

## Security Principles

- Production install roots and trusted upstreams are hard-coded.
- Secrets must stay out of agent-readable plaintext.
- Approval should happen at meaningful risk boundaries, not just command names.
- Metadata can describe risk, but it must not become a runtime plugin system.
- GUI and CLI surfaces should explain why an action is risky before asking for
  approval.

## UX Principles

- Every click must produce visible feedback.
- Long work needs loading, success, error, and retry states.
- Package pages should keep the user oriented: where am I, what package is this,
  and what action am I about to allow?
- External links should be explicit destinations, not hidden URL rewrites.
- Security status must not rely on color alone.

