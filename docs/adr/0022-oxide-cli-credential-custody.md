# ADR 0022: Patch Oxide CLI for profile-bound credential custody

Status: accepted

## Context

Oxide CLI stores reusable profile tokens in
`~/.config/oxide/credentials.toml`. Upstream provides no external credential
helper, so a config wrapper would have to recreate plaintext before every run.
The CLI is a single Rust executable and can instead be patched at its credential
boundary.

## Decision

The Oxide CLI Hardener installs an Automic Vault Isotope built from a pinned,
SHA-256-verified upstream source archive. The reviewed patch replaces supported
profile tokens with the reserved non-secret `@av` marker and routes token reads,
login stores, and single-profile logout deletes through fixed Oxide-only XPC
operations. `OXIDE_TOKEN`, plaintext tokens, unknown credential fields, and
bulk logout fail closed.

Each Secret Name is `OXIDE_PROFILE_TOKEN_` plus the uppercase SHA-256 digest of
the canonical profile, a NUL separator, and the canonical HTTP(S) host. The
Hardener stores every token with save-if-absent-or-equal semantics before
atomically replacing the mode-0600 configuration file. Failure leaves the
original configuration available for recovery.

The release workflow builds the patched executable with the pinned upstream
Rust toolchain, signs it as identifier `oxide` under Automic Vault team
`ZU76A67LGU` with Hardened Runtime and a trusted timestamp, rejects embedded
entitlements, and publishes it from this repository. The privileged installer
accepts only the expected archive entry and revalidates the release digest,
signature, runtime, timestamp, and entitlements before installing
`/usr/local/bin/oxide`.

For every credential operation, the menu app derives the helper's live parent
from the kernel and requires that exact installed Target, Developer ID identity,
Hardened Runtime, process lifetime, arguments, profile, host, and Secret Name.
Those values are bound into the complete Authorization Request and revalidated
immediately before Secret Application or mutation. Unknown commands fail
closed in the Authorization Gate classifier.

## Consequences

Oxide profile tokens remain under Automic Vault custody at rest and are not
recreated in config files. Arbitrary same-user processes cannot use the helper
without a live eligible Oxide Target and an Authorization Decision.

The authorized Oxide Target necessarily receives its reusable token in memory.
Automic Vault cannot make an authorized or compromised Target keep that token
confidential after release. Automic Vault also assumes responsibility for the
patched Target and its update path; upstream schema or authentication changes
require a reviewed patch update.
