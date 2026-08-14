# ADR 0013: Generate Launcher Bundles for Unsigned CLI Executables

- Status: Proposed
- Date: 2026-08-14

## Context

Unsigned and arbitrary ad-hoc-signed CLI executables cannot supply a persistent
Launcher Identity. Their files are user-mutable, they have no stable publisher
identity, and without Hardened Runtime another same-user process may be able to
alter their execution. Path or filename matching would turn ordinary filesystem
control into durable Developer Authority.

Users nevertheless need long-running CLI executables and agent harnesses to
participate in Authorization Gate policy without approving every operation.
Treating those executables as Blessings, Authorization Gates, or Isotopes would
conflate distinct security boundaries.

## Decision

Automic Vault may generate a Launcher Bundle containing one fixed CLI payload.
The first implementation accepts one regular Mach-O executable. It does not
accept scripts or directory-shaped tools.

The bundle's launcher executable remains the payload's parent process, forwards
its arguments and result, and executes only the payload sealed inside its
bundle, never a command resolved through `PATH`.

Automic Vault signs nested code inside-out and enables Hardened Runtime. It uses
either an Automic-created ad-hoc signature or a Developer ID Application
identity selected by the user. Enrollment occurs only through an attended
Automic Vault flow.

Creation and enrollment are one attended transaction in the **Launcher
Bundles** sidebar. Automic Vault builds and verifies a candidate in private
temporary storage, shows the selected payload digest, signing identity, and
effective entitlements, then atomically installs and enrolls it after the user
confirms. Failed or abandoned candidates do not become sidebar entries.

Enrollment binds an Automic-generated marker, a new per-generation Launcher
Identity, the final bundled payload's SHA-256 digest, and its accepted Launcher
Runtime Requirement. The selected source path and pre-signing digest are review
inputs only; they are not enrolled identity or monitored state. Automic Vault
verifies the live Launcher, bundle seal, payload digest, enrollment, and runtime
posture on every Authorization Request. Missing or conflicting evidence fails
closed. An arbitrary ad-hoc-signed app cannot enter this path merely by
imitating the bundle layout.

After successful enrollment, the bundle's live launcher process can qualify as
a Verified Launcher. It uses the existing Authorization Policy model without
special Access Levels: gate defaults and Launcher-specific rules apply normally.
It also uses the existing runtime eligibility matrix, including its accepted
compatibility exceptions, warnings, and blocked entitlements.

Every payload update creates a new bundle identifier and designated
requirement. Existing Launcher-specific rules therefore do not carry to the new
generation. The user must explicitly enroll the replacement, after which gate
defaults apply until the user grants narrower or broader Launcher-specific
rules.

## Consequences

- Automic Vault must keep Launcher Bundle enrollment evidence separately from
  Authorization Policy and reject generated bundles whose evidence is missing
  or corrupt.
- A user-selected Developer ID identity does not identify the CLI publisher and
  never replaces payload pinning.
- The original unbundled executable remains a different, unverified Launcher
  and cannot match the Launcher Bundle's enrollment or Launcher-specific rules.
- Sealing and Hardened Runtime reduce mutation and injection risk. They do not
  make the payload, its prompts, configuration, plug-ins, extensions, or child
  processes trustworthy.
- Supporting scripts or multi-file tools requires a separate design for
  interpreter and dependency identity.
