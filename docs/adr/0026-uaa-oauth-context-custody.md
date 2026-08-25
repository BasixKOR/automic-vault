# ADR 0026: UAA OAuth context custody

Status: Accepted

## Context

UAA CLI is a single Go executable that persists access and refresh tokens for
multiple targets and contexts in `~/.uaa/config.json` (or `$UAA_HOME/config.json`).
It provides no credential-helper interface. A wrapper would have to recreate a
plaintext config file and would therefore not establish a durable Secret
Custody boundary.

The official macOS release is not signed with an identity suitable for the
Automic Vault Secret Gate. Its token refresh and context-management commands
also update the same persisted config they read.

## Decision

The `automic-vault/uaa-cli` fork publishes a pinned, patched UAA CLI Isotope
signed with Developer ID, Hardened Runtime, timestamping, and no entitlements.
The signed Isotopes tap pins the exact fork release URL and digest as specified
by [ADR 0029](0029-fork-owned-isotope-releases.md). The patch is limited to the
upstream config read/write boundary. It stores one strictly validated map of
target/context OAuth tokens through dedicated XPC operations and leaves only
`@av` markers plus non-secret metadata on disk.

The approval service binds every helper operation to the live signed `uaa`
parent, its complete arguments, the fixed credential scope, and the exact
Secret Name. Unknown commands and arbitrary `uaa curl` requests fail closed.

## Consequences

UAA CLI remains compatible with its normal target and context lifecycle without
writing reusable OAuth tokens to its config. Upstream version, commit, source
digest, patch, signing identity, runtime flags, entitlements, archive contents,
and provenance are verified during release and installation.
