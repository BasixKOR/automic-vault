# ADR 0019: Keep GPG signing keys behind a Tool-specific Authorization Gate

Status: accepted

## Context

Git's `gpg.program` interface sends a commit or tag payload to a GPG-compatible
process and expects a detached OpenPGP signature. Ordinary GnuPG configurations
let a same-user process ask the agent to sign, so possession of the user session
often becomes ambient signing authority. Giving a wrapper the exported private
key would merely move that ambient authority into another file or process. Git
configures a program pathname rather than a subcommand and uses that same
program for signing and verification.

Users also need agent-authored commits to be distinguishable from commits made
through their normal interactive Launcher. A client-provided “agent” flag is
forgeable and cannot safely select the more valuable default credential.

## Decision

Bundle `av-gpg` as Git's narrow GPG-compatible adapter inside
`Automic Vault.app` without installing a second system command. For signing
requests, `av-gpg` executes the adjacent signed `av gpg-sign` Command and
streams the immutable payload. `av` owns the OpenPGP implementation, holds the
bounded payload in memory, and binds its SHA-256 digest into the Authorization
Request. `av-gpg` receives only the detached signature. Verification and
unrelated GPG operations continue to delegate to the user's `gpg` command.

The menu app treats each request as Local Write at the GPG Signing Gate. It
verifies `av` as Gate Client and Target, resolves the live Verified Launcher,
and selects one GPG Signing Credential before authorization. The default and
alternate credentials each contain a private-key Secret and passphrase Secret
in the Data Protection Keychain.

Launcher Signing Credential Rules are stored in the Data Protection Keychain
and bind exact designated requirements to the alternate credential. The menu
app derives selection from the verified process chain; no client-controlled
field can select a credential. Every rule mutation uses the existing approved
authority-change flow. If a matching rule exists but its alternate credential
is unavailable, signing fails closed without using the default.

After an allowed Authorization Record is persisted and verified, the app
releases exactly the selected credential to the signed `av` Target. `av` parses
the OpenPGP transfer key, creates an ASCII-armored detached signature in memory,
and zeroizes its transient input buffers. Payloads and keys are bounded to 16
MiB. Git and `av-gpg` never receive credential bytes.

## Consequences

Using a signing key now requires an Authorization Decision over the Verified
Launcher, Gate Client, Target, arguments, working directory, selected Secret
Names, and process execution. The gate defaults to Read Only, so Local Write
signing requires Approval until policy explicitly grants it.

The signed `av` Target necessarily handles usable private-key material in its
memory. Hardened Runtime and code-signature verification reduce injection risk
but do not make the Target immune to a same-user or kernel compromise. OpenPGP
keys whose primary key cannot sign are rejected rather than heuristically
choosing a potentially revoked or expired subkey.
