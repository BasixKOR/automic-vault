# ADR 0013: Generate Launcher Bundles for Unsigned CLI Executables

- Status: Accepted
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
effective entitlements, then installs it under
`~/Applications/Automic Vault/` and enrolls it after the user confirms. The
transaction fails closed: failed or abandoned candidates do not become sidebar
entries or recognized Launchers. Before enrollment, Automic Vault validates the
completed bundle strictly, including its nested code and every architecture.

Enrollment binds a reserved Automic-generated identifier and generation, the
new per-generation Launcher Identity, the Security framework's exact signed
code identifiers for the completed outer bundle, the final bundled payload's
SHA-256 digest, its signing type, and its accepted Launcher Runtime Requirement.
For a multi-architecture bundle, enrollment records the exact code identifier
for every architecture. The selected source path and pre-signing digest are
review inputs only; they are not enrolled identity or monitored state.

Enrollment evidence lives in an app-private Data Protection Keychain item,
separate from Authorization Policy. Only the attended Automic Vault creation
flow may mutate it; no general CLI or XPC enrollment operation is exposed.

Every Authorization Request from a Launcher Bundle revalidates the live
Launcher and the static bundle through the Security framework before using
their signing information. Validation is strict and covers nested code and all
architectures. Automic Vault then compares the executing architecture's exact
signed code identifier, the reserved generation and Launcher Identity, a
safe-snapshot SHA-256 of the final bundled payload, and the live runtime posture
with enrollment. It does not parse code-signing internals itself.

The launcher starts the fixed payload suspended, compares the child process's
live code identifier with the payload identifiers sealed into the launcher's
own signed code, and resumes it only after a match. This binds validation to the
process that will execute instead of relying on a path check followed by an
unchecked path execution.

Anything claiming the reserved Launcher Bundle identifier namespace enters
this verification path. Missing, corrupt, or conflicting evidence hard-denies
the request and cannot fall through to Approval or ordinary Developer ID or
ad-hoc Launcher eligibility. An arbitrary ad-hoc-signed app therefore cannot
enter this path merely by imitating the bundle layout.

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

Replacement builds and verifies the new generation without disturbing the old
one. Its enrollment update activates the new generation and revokes the old one
as one persisted state change. If that update fails, the old generation remains
enrolled. After it succeeds, Automic Vault removes the old generation's
Launcher-specific rules and moves its bundle to the user's Trash. A failure to
move the old file does not restore its enrollment or authority.

## Consequences

- Automic Vault must keep Launcher Bundle enrollment evidence separately from
  Authorization Policy and reject generated bundles whose evidence is missing
  or corrupt.
- The ad-hoc signature supplies a code seal and Hardened Runtime posture, not a
  durable publisher identity. Re-signing changed code changes its exact signed
  code identifier and invalidates enrollment. Developer ID-signed generations
  are pinned the same way.
- A user-selected Developer ID identity does not identify the CLI publisher and
  never replaces payload pinning.
- The original unbundled executable remains a different, unverified Launcher
  and cannot match the Launcher Bundle's enrollment or Launcher-specific rules.
- Sealing and Hardened Runtime reduce mutation and injection risk. They do not
  make the payload, its prompts, configuration, plug-ins, extensions, or child
  processes trustworthy.
- Same-user malware can still delete or damage a Launcher Bundle and cause
  denial of service. It cannot change the enrolled code while retaining that
  generation's authority. A bit-for-bit restore at the enrolled path remains
  the same code; copies at other paths are denied.
- Supporting scripts or multi-file tools requires a separate design for
  interpreter and dependency identity.
