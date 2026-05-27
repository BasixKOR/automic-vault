# Automic Vault Features

Automic Vault is a local macOS security layer for AI coding agents. It keeps
developer secrets out of plaintext files and model context, injects approved
credentials only into trusted command-line tools, and adds human approval gates
where those tools run.

## Core Feature Areas

| Feature | What it does | User value |
| --- | --- | --- |
| Nucleus package manager | Installs Homebrew formulae, casks, npm packages, PyPI packages, vendor tools, and Automic Vault isotopes under controlled roots. | Gives agents useful tools without making the developer environment ambient writable state. |
| Controlled install roots | Release builds install packages under `/opt` and expose stubs through `/usr/local/bin`; debug builds use `/tmp/opt` and `/tmp/usr/local/bin`. | Keeps package ownership and execution paths predictable for both humans and agents. |
| Package catalog search | Searches installed packages and the available vault catalog, with prefix-aware names for Homebrew, casks, isotopes, npm, PyPI, vendor, system, and detected-gone packages. | Lets users find the real package surface instead of guessing which ecosystem owns a tool. |
| Package dossier | Shows package source, version state, description, aliases, dependencies, executable paths, popularity, last update date, install destination, security notices, and action commands. | Gives enough context to install, update, remove, migrate, or remediate a package deliberately. |
| External package surface | Embeds the package homepage or GitHub README/release page in a passive WebKit surface, with links opened explicitly in the browser. | Keeps upstream context visible while preserving app focus and avoiding hidden navigation surprises. |
| Recommendations | Surfaces setup tasks such as installing the `av` command-line tool, installing Xcode Command Line Tools, installing agent-oriented package packs, and migrating Homebrew packages. | Turns first-run and maintenance gaps into visible, actionable rows in the app. |
| Updates and outdated checks | Lists Nucleus and Homebrew packages with newer versions, shows app update availability, and runs privileged update operations through the helper. | Keeps the agent toolchain current without hiding which packages are changing. |
| Homebrew migration | Detects Homebrew installs that can move under Automic Vault control, displays hazards, installs replacements, and removes original Homebrew packages when migration completes. | Moves agent-used tools from ambient Homebrew roots into managed vault roots. |
| Isotopes | Uses secured forks of open source tools with explicit approval gates and optional secret migration at risky execution points. | Moves security boundaries into the tools agents actually run. |
| Radioisotopes and detectors | Uses package metadata and detectors to identify plaintext secret exposure before a full isotope exists. | Provides early warnings and remediation paths for known package risks. |
| Local hazard detection | Promotes detected local secret exposure as `sys:` or `gone:` package rows when a detector finds risk outside the installed vault set. | Makes exposed system and previously installed tool credentials visible in the same package workflow. |
| Approval gates | Blocks manual gate requests, command execution requests, and isotope secret injection requests until a user approves or denies them. | Prevents agents from silently publishing, deleting, mutating infrastructure, or receiving secrets. |
| Containment | Runs an agent command inside a generated sandbox and proxy toolchain, routing tool execution through the vault approval daemon. | Adds a practical review layer below the agent session. |
| Secrets keychain | Stores secrets in the Automic Vault keychain and injects them into approved processes. | Keeps credentials out of `.env`, shell startup files, shell history, and model context. |
| Credential helper adapter | Provides an approved credential-helper path for tools that need credentials at runtime. | Lets existing tools request credentials through a controlled boundary. |
| Secret scanner | Scans isotope detectors and likely local secret files for plaintext credentials, with human, JSON, and JSONL output. | Finds credentials that are already visible to agents before they are used or leaked. |
| Static trace | Asks a local Codex or Claude agent to explain likely file-changing steps in a shell one-liner without running it. | Helps users understand installer and command risk before execution. |
| Local protocol daemon | Starts the read-only `av serve` protocol used by the app for package status, search, detail, outdated, pulse, and migration queries. | Keeps package metadata available locally without making install roots or trusted upstreams runtime-configurable. |
| Privileged helper | Performs install, update, uninstall, make-default, isotope conversion, CLT install, database refresh, and app update operations after biometric authorization. | Keeps privileged package mutations explicit and interruptible from the GUI. |
| macOS app | Provides the native AppKit package console, package dossier, external surface, command palette, update overlay, and approval prompts. | Makes agent security and package operations inspectable without requiring terminal fluency. |
| Menu bar helper | Shows installed and outdated status, app update state, hazardous-package indication, last refresh errors, start-at-login control, approval notifications, and auto-approved secret toasts. | Gives lightweight operational feedback and opens the main app only when needed. |

## Current Product Shape

Automic Vault has four primary user surfaces:

1. `av` CLI for package, secret, approval, containment, trace, and protocol
   daemon workflows.
2. Automic Vault.app for package browsing, package dossiers, security notices,
   recommendations, external package context, privileged package operations,
   and approval prompts.
3. The menu bar helper for status refresh, notifications, launch control, and
   quick access to the main app.
4. Repository-maintained isotope, radioisotope, package approval, and package
   enrichment metadata that describes package-specific security behavior.

## App Workflow

The current app is built around a single package console:

1. Start with installed packages, recommendations, and pulse/discovery rows.
2. Type to search installed tools and the vault catalog; begin with `>` to use
   command-palette views such as all packages or pulse packages.
3. Select a row to load its dossier and external package surface.
4. Review source, version, install state, dependency, executable, destination,
   security, and homepage context.
5. Take the visible action: install, update, uninstall, make default, migrate
   from Homebrew, reinstall as isotope, migrate isotope secrets, install CLT,
   or update the app.

Rows may represent installed vault packages, available packages, setup
recommendations, Homebrew migration candidates, unsupported Homebrew installs,
detected local hazards, system detector findings, or command-palette entries.

## CLI Workflows

The `av` command groups its workflows by boundary:

- Package system: `install`/`i`, `info`, `search`, `list`/`ls`, `outdated`,
  `update`/`up`, and `uninstall`/`rm`.
- Access control: `scan`, `inject`, `save`, and `credential-helper`.
- Execution control: `contain`, `trace`, and `gate`.
- Local system: `serve`.

Package status and search commands support machine-readable output where the
runtime exposes JSON or JSONL. Mutating package commands acquire the package
mutation lock and run through root or helper-controlled paths where required.

## Approval Experience

Automic Vault has two approval prompt shapes:

- Command execution approvals show the command line, requesting process, current
  directory, agent id, and environment keys.
- Isotope secret approvals show requested secret names, the command that will
  receive them, parent process, requested executable, audited executable,
  interpreter script information, root-control status, and whether "always
  allow" is available.

Approval should happen at the meaningful risk boundary: the executable,
script, secret, command, or infrastructure action being requested.

## Security Principles

- Production install roots and trusted upstreams are hard-coded.
- Secrets must stay out of agent-readable plaintext.
- Approval should happen at meaningful risk boundaries, not just command names.
- Metadata can describe risk, but it must not become a runtime plugin system.
- GUI and CLI surfaces should explain why an action is risky before asking for
  approval.
- Privileged operations require visible progress and an explicit authorization
  path.

## UX Principles

- Every click must produce visible feedback.
- Long work needs loading, progress, success, error, and retry states.
- Package rows should make their source and state legible without relying only
  on color.
- Package pages should keep the user oriented: where am I, what package is this,
  and what action am I about to allow?
- The external surface should support package inspection, while deliberate link
  activation opens the browser.
- Approval prompts should present command, process, environment, target, and
  secret context before a decision is requested.
