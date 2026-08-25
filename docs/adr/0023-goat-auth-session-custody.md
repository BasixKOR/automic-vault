# ADR 0023: Patch goat for DID- and PDS-bound credential custody

Status: accepted

## Context

goat stores its Bluesky app password, access token, and refresh token in
`$XDG_STATE_HOME/goat/auth-session.json`. Upstream provides no external
credential helper, so a config wrapper would have to recreate plaintext before
every run. goat is a single Go executable and can instead be patched at its
credential boundary.

## Decision

The goat Hardener installs an Automic Vault Isotope built from a pinned,
SHA-256-verified upstream source archive. The reviewed patch replaces the three
supported secret fields with the reserved non-secret `@av` marker and routes
session reads, login and refresh stores, and logout deletes through fixed
goat-only XPC operations. Unknown, incomplete, partially migrated, and trailing
JSON data fail closed.

Each Secret Name is `GOAT_AUTH_SESSION_` plus the uppercase SHA-256 digest of
the canonical DID, a NUL separator, and the canonical HTTP(S) PDS origin. The
Hardener stores a complete session secret with save-if-absent-or-equal semantics
before atomically replacing the mode-0600 state file. Failure leaves the
original state available for recovery.

The `automic-vault/goat` fork release builds the patched executable from the
pinned upstream commit with the pinned Go toolchain, signs it as identifier
`goat` under Automic Vault team `ZU76A67LGU` with Hardened Runtime and a trusted
timestamp, rejects embedded entitlements, and publishes it from that fork. The
signed Isotopes tap pins the exact fork release URL and digest. The privileged
installer accepts only the expected archive entry and revalidates the manifest,
digest, signature, runtime, timestamp, and entitlements before installing
`/usr/local/bin/goat`, as specified by
[ADR 0029](0029-fork-owned-isotope-releases.md).

For every credential operation, the menu app derives the helper's live parent
from the kernel and requires that exact installed Target, Developer ID identity,
Hardened Runtime, process lifetime, arguments, DID, PDS, and Secret Name. Those
values are bound into the complete Authorization Request and revalidated
immediately before Secret Application or mutation. Unknown commands fail closed
in the Authorization Gate classifier.

## Consequences

goat session credentials remain under Automic Vault custody at rest and are not
recreated in state files. Arbitrary same-user processes cannot use the helper
without a live eligible goat Target and an Authorization Decision.

The authorized goat Target necessarily receives its reusable session secrets in
memory. Automic Vault cannot make an authorized or compromised Target keep those
values confidential after release. Command-line and environment credentials
supplied to explicit login commands remain outside this stored-session boundary.
Automic Vault also assumes responsibility for the patched Target and its update
path; upstream schema or authentication changes require a reviewed patch update.
