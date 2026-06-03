# Automic Vault Security Methods and Gaps

Last updated: June 3, 2026

## Abstract

Automic Vault is a local macOS security layer for AI coding agents. It does not
try to make a model safe, and it does not claim that local tools become safe
after approval. Its purpose is narrower: move developer secrets out of files an
agent can read, make powerful command execution visible, keep package roots
predictable, and put human approval at the point where a local tool receives
authority.

This paper describes the current methods, the parts that are working, and the
holes that remain. It is written against the public source tree and product
shape as of June 3, 2026.

## Executive Summary

Automic Vault is strongest when the risk is ambient local authority: plaintext
tokens in `.env`, `~/.aws/credentials`, `~/.npmrc`, GitHub CLI state, MCP
configuration, or other files that an agent can read before a human notices.
It stores supported secrets in the macOS keychain, asks before injecting named
values into an executable, and can run an agent through a synthetic toolchain
where host tool execution is mediated by the local app.

The design succeeds at moving several critical decisions below the prompt
layer. An agent cannot simply read a key from a migrated plaintext file if the
key now lives behind keychain-backed injection. An approval prompt can show the
requested secret names, executable path, command line, parent process, current
directory, and root-control status. Always-allow is intentionally narrow: it is
available for root-owned, non-writable executables, root-owned scripts, or
hash-bound scripts behind root-controlled interpreters.

The design is not a complete sandbox. A same-user malicious process can attack
local per-user state. Root or kernel compromise bypasses the model. An approved
tool receives the secret and can still misuse it. `av contain` currently gates
child process execution more than file or network access. Package and metadata
trust still depends heavily on upstream ecosystems unless a package has a
specific isotope or detector. These are important boundaries, not footnotes.

## Threat Model

Automic Vault assumes a local AI coding agent can:

- Read project files and common developer configuration files.
- Invoke command-line tools from the user's shell environment.
- Cause sensitive strings to appear in transcripts, logs, patches, or tool
  output.
- Follow misleading instructions from a repository, issue, web page, package
  README, or terminal output.
- Use installed package managers and CLIs with the same ambient state a human
  developer would normally have.

Automic Vault also considers a compromised or malicious package tool that tries
to read local credentials or ask another command to perform a sensitive action.

Out of scope today:

- A malicious process that already has the same macOS user account and is
  deliberately modifying Automic Vault's per-user approval files or socket.
- A root, kernel, hypervisor, or physical-device compromise.
- Cloud-side identity compromise after a credential is legitimately used.
- A model that exfiltrates data the user intentionally pasted into the prompt.
- A fully malicious approved executable. Approval is a capability grant, not a
  proof that the executable is safe.

## Architecture

The main security boundary is local, not hosted.

- `Automic Vault.app` is the AppKit GUI for package dossiers, security states,
  approval prompts, helper operations, and containment logs.
- `av` is the CLI for package operations, scanning, secret save/inject,
  containment, tracing, and approval gates.
- `nuke-helper` is the privileged helper for operations that need root
  privileges, including package installs, updates, uninstall, isotope
  conversion, helper maintenance, and root-owned always-allow records.
- The macOS keychain stores named CLI secrets under the Automic Vault isotope
  service.
- Per-user approval requests and decisions live under
  `~/Library/Application Support/Automic Vault/`.
- Release package roots are fixed at `/opt` with stubs in `/usr/local/bin`.
  Debug builds use `/tmp/opt` and `/tmp/usr/local/bin`.
- Production upstream roots and install locations are hard-coded rather than
  runtime-configurable.

The website is static product information and download metadata. It is not a
hosted vault, account system, or remote policy engine.

## Secret Storage

`av save KEY` stores a trimmed value in the macOS keychain. Key names are
restricted to ASCII identifier-like names. Empty keys and empty trimmed values
are rejected.

New keychain items are created with a trusted-application ACL that includes the
current application context and `/usr/local/bin/av` when that path exists and is
executable. Existing items are updated without changing the ACL because changing
access on an existing keychain item can trigger a system authorization dialog.

This is a useful improvement over plaintext files, but it is not magic. The
keychain protects the stored item according to macOS keychain rules. Once a
secret is loaded for injection, it exists in process memory. Once injected, it
exists in the approved process environment.

## Secret Injection

`av inject +KEY /absolute/path/to/tool ...` resolves and opens the target,
checks that it is a regular file, validates parent directory writability, asks
for approval when a requested key is not already allowed, then executes the
target with selected environment variables added.

Several details matter:

- The target path must be absolute.
- `av inject` refuses to run as root.
- Core dumps are disabled before key loading.
- Existing environment variables are preserved unless
  `--replace-existing-env` is used.
- Missing keys can be skipped only when `--allow-missing-keys` is explicitly
  requested.
- The target opened for validation is compared with the path immediately before
  `execve` to reduce time-of-check/time-of-use replacement.
- On failure paths, loaded credential strings are zeroed before being dropped.

The approval prompt is intentionally about named keys and executable context,
not raw secret values. The model should never receive the secret just so it can
paste it into a command.

## Always-Allow Rules

Always-allow is deliberately narrower than "remember this prompt."

It is available when the executable boundary is root-controlled: the executable
must be owned by root, not group- or world-writable, executable, and reached
through directories that are not group- or world-writable. For interpreters such
as `sh`, `bash`, `zsh`, `python`, `node`, `ruby`, `perl`, and `osascript`,
the script file is part of the approval scope.

Root-owned scripts can be remembered by path. Non-root scripts can be remembered
only by path plus SHA-256 hash. `env` shebangs and `env` launchers are not
eligible because they hide the true interpreter resolution behind runtime
environment lookup.

This is one of the better parts of the current design. It moves durable
approval away from mutable project files and toward root-owned or hash-bound
execution boundaries.

## Command Approval

Automic Vault has two approval families:

- Isotope secret approvals for named secret injection into a command.
- Command execution approvals for contained host-tool execution or manual
  `av gate` requests.

Approval requests include the command, current directory, parent or requesting
process, agent id where available, and environment key names. Decisions are
matched by request id and cleared after use.

The weak point is that these channels are local per-user coordination
mechanisms, not a cryptographic authorization service. A malicious process
already running as the same macOS user may be able to interfere with per-user
approval state. This is acceptable for accidental agent overreach and many
compromised-tool scenarios, but it should not be described as same-user malware
containment.

## Containment

`av contain <command>` builds a temporary toolchain in `/tmp`, replaces `PATH`
with generated stubs, starts the initial command through `sandbox-exec`, and
routes stub execution to the local vault daemon for approval. The daemon can
approve, execute the host tool, and stream stdout/stderr and exit status back
to the contained process.

The current sandbox profile uses `allow default` followed by a denial of
`process-exec`, with explicit process-exec allowances for the initial command
and the generated vault proxy toolchain. That means the current containment
model is best understood as execution mediation. It is not a file-read sandbox,
not a network sandbox, and not a full macOS application container.

This control still helps. Many agent hazards involve running `gh`, `aws`,
`npm`, `docker`, `kubectl`, `terraform`, `curl`, or another local tool. Putting
an approval point between the agent and host tool execution is valuable. It just
does not stop the initial contained process from reading files that the sandbox
still allows.

## Package Roots and Metadata

Nucleus installs supported packages under predictable roots and exposes command
stubs through predictable paths. Release builds use `/opt` and
`/usr/local/bin`; debug builds use `/tmp/opt` and `/tmp/usr/local/bin`.
Production package roots and trusted upstream API roots are not runtime
configuration surfaces.

The package catalog, isotope metadata, radioisotope detectors, and security
recommendations let the app explain package-specific risk: readable credential
paths, hardening availability, approval gates, known hazardous local state, and
stale packages.

This is a practical security win because the user can inspect a concrete
package, path, version, executable, and remediation instead of reasoning about a
generic "agent risk."

The gap is supply-chain trust. Automic Vault reduces local credential exposure
and can harden specific tools, but it is not yet a universal provenance system
for every upstream package, bottle, tarball, wheel, or npm artifact. A
compromised upstream may still execute code during install or runtime unless
the relevant path is covered by an isotope, detector, approval gate, or a
future stronger provenance policy.

## Privileged Helper

The helper exists because release package operations need root-controlled
paths. The GUI uses Apple's privileged-helper flow to install or update the
helper, asks for device-owner authentication before privileged operations, and
checks helper identifier, team identifier, and helper version when deciding
whether blessing is needed.

The helper verifies its own expected code-signing identity at startup and again
when privileged commands are invoked. CLI installation compares the helper,
GUI caller, and staged CLI tool signatures before installing `/usr/local/bin/av`.
The helper also sanitizes selected inherited environment variables that could
change package-manager behavior.

The caveat is caller authorization at the helper boundary. The app performs the
human authentication and uses typed XPC interfaces, but the helper listener is
not yet documented here as a complete per-client authorization firewall for
every method. Future hardening should make caller identity checks explicit in
the helper service itself, not only in app-side flows and selected operations.

## Privacy and Telemetry

Product secrets are local. They are stored in the macOS keychain and injected
into approved local processes. The website is static and does not host user
vaults.

Release app builds include a PostHog telemetry path for a main-window-opened
event when an API key is present in the bundle. That event uses an anonymous
install id and app/system properties such as app version, build id, protocol
version, helper version, macOS version, machine architecture, Mac model,
processor counts, and rounded physical memory. It does not include secret
values, package tokens, command lines, approval payloads, or local file paths.

This should remain narrow. A security product should be conservative about
telemetry, and any expansion should be opt-in or very clearly disclosed.

## Successes

Automic Vault has already made several good security choices:

- It treats prompt-level safety as insufficient and puts controls at local
  runtime boundaries.
- It uses macOS keychain storage instead of inventing its own secret database.
- It refuses durable approval for mutable, easy-to-rewrite execution paths.
- It presents concrete approval context: command, executable, parent process,
  current directory, requested keys, script status, and root-control status.
- It keeps release install roots and package metadata endpoints fixed.
- It uses a privileged helper for root-owned mutations instead of asking the
  GUI to run as root.
- It says the quiet part out loud: this does not make agents safe.

## Holes and Caveats

The current design should be presented with these limitations:

- Same-user malware is not contained. A malicious process running as the same
  user is outside the current approval-file and socket trust model.
- Root compromise wins. Root can read or modify package roots, helper state,
  approval records, and process memory.
- Approved tools can leak. A secret injected into `aws`, `gh`, `npm`, or any
  other approved executable is available to that process and may be available
  to children it spawns.
- Environment variables are a leaky transport. They are convenient for CLIs,
  but they are not the best long-term secret delivery mechanism.
- Containment is mostly process-exec mediation today. It is not a strong file,
  network, or syscall sandbox.
- Scanner and detector coverage is incomplete. New tools, custom configs, and
  unusual paths can be missed.
- Package provenance is not universal. Upstream package integrity still depends
  on the package ecosystem unless Automic Vault has package-specific hardening.
- Approval UX can be habituating. If prompts become too frequent or too broad,
  users will approve mechanically.
- There is no formal public third-party audit documented on the website as of
  this paper.

## Operational Guidance

Use Automic Vault where it is strongest:

1. Run the scanner before giving an agent broad repository access.
2. Move supported secrets out of plaintext config and into `av save`.
3. Prefer root-owned tools or root-owned scripts for durable approvals.
4. Treat every approval as a capability grant to a specific executable and
   command path.
5. Use `av contain` to mediate host tool execution, but do not treat it as a
   complete sandbox.
6. Keep high-value production credentials outside routine agent workflows when
   possible.
7. Review package hazards after new installs and after updating agent tools.

## Hardening Roadmap

The most valuable next hardening work is:

- Authenticate local approval channels more strongly than writable per-user
  files.
- Add explicit helper-side client identity and authorization checks for every
  privileged method.
- Reduce environment-variable secret transport for tools that can support file
  descriptors, keychain callbacks, credential-helper protocols, or scoped IPC.
- Expand containment from process-exec mediation toward more granular file and
  network policy where macOS allows it.
- Sign and verify package/security metadata with rollback protection.
- Add more package-specific isotopes, detectors, and approval manifests for
  high-risk developer tools.
- Publish a formal external security review when the core boundary stabilizes.

## Conclusion

Automic Vault is a practical local boundary for a real new problem: agents can
read the same local files and run the same local tools developers use. Its
strongest controls are concrete and useful: keychain-backed storage,
approval-scoped injection, root-controlled durable approvals, predictable
package roots, and visible local command mediation.

The honest claim is not that Automic Vault makes agents safe. The honest claim
is that it removes several high-value ambient privileges and makes sensitive
local authority harder to spend silently. That is worth shipping, and the
remaining gaps are clear enough to guide the next round of hardening.
