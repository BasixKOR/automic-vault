# ADR 0024: Patch Railway CLI for environment-bound credential custody

Status: accepted

## Context

Railway CLI stores either a legacy token or an OAuth access token and optional
refresh token in production, staging, and development config files under
`~/.railway`. OAuth refresh and logout read and rewrite that shared state.
Upstream provides no external credential helper, so a config wrapper would
either recreate plaintext or fail to cover rotation and deletion. Railway CLI
is a single Rust executable and can instead be patched at the shared config
boundary.

## Decision

The Railway Hardener installs an Automic Vault Isotope built from a pinned,
SHA-256-verified upstream source archive. The reviewed patch replaces supported
secret fields with the reserved non-secret `@av` marker and routes config reads,
OAuth refresh stores, and logout deletes through fixed Railway-only XPC
operations. Unknown credential fields, mixed legacy and OAuth credentials,
incomplete credentials, and partially migrated state fail closed.

Each Secret Name is `RAILWAY_AUTH_` plus the uppercase SHA-256 digest of the
fixed environment name, a NUL separator, and its fixed Railway host. The
Hardener validates all three config files and stores every credential with
save-if-absent-or-equal semantics before atomically replacing any file with its
mode-0600 marker form. Failure before replacement leaves the original files
available for recovery.

The `automic-vault/railway-cli` fork release builds the patched executable from
the pinned upstream commit with a pinned Rust toolchain, signs it as identifier
`railway` under Automic Vault team `ZU76A67LGU` with Hardened Runtime and a
trusted timestamp, rejects embedded entitlements, and publishes it from that
fork. The signed Isotopes tap pins the exact fork release URL and digest. The
privileged installer accepts only the expected archive entry and revalidates
the manifest, digest, signature, runtime, timestamp, and entitlements before
installing `/usr/local/bin/railway`, as specified by
[ADR 0029](0029-fork-owned-isotope-releases.md).

For every credential operation, the menu app derives the helper's live parent
from the kernel and requires that exact installed Target, Developer ID identity,
Hardened Runtime, process lifetime, arguments, environment, host, and Secret
Name. Those values are bound into the complete Authorization Request and
revalidated immediately before Secret Application or mutation. Unknown
commands fail closed in the Authorization Gate classifier.

## Consequences

Supported Railway session credentials remain under Automic Vault custody at
rest and are not recreated in config files. Arbitrary same-user processes
cannot use the helper without a live eligible Railway Target and an
Authorization Decision.

The authorized Railway Target necessarily receives its reusable credential in
memory. Automic Vault cannot make an authorized or compromised Target keep it
confidential after release. Explicit `RAILWAY_TOKEN` and `RAILWAY_API_TOKEN`
environment variables remain outside this stored-session boundary. Automic
Vault also assumes responsibility for the patched Target and its update path;
upstream schema or authentication changes require a reviewed patch update.
