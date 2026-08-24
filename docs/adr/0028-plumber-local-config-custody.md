# ADR 0028: Plumber local-config custody

Status: Accepted

## Context

Plumber is a single Go executable that stores service tokens and nested backend
connection credentials in `~/.batchsh/plumber.json`. It has no credential-helper
interface, and normal connection, relay, and tunnel updates rewrite the same
config. A wrapper would therefore recreate plaintext credentials on disk.

The nested connection schema is generated and covers many backends. Maintaining
a second field-by-field secret schema in Automic Vault would create an omission
risk whenever upstream adds a credential-bearing field.

## Decision

Automic Vault publishes a pinned Plumber Isotope signed with Developer ID,
Hardened Runtime, timestamping, and no entitlements. The upstream patch is
limited to the local config read/write boundary: it stores the complete local
config JSON through dedicated XPC operations and persists only a fixed custody
marker. Cluster-mode KV storage is unchanged.

The approval service binds helper operations to the live signed `plumber`
parent, its complete arguments, the fixed local-config scope, and the exact
Secret Name. Unknown commands fail closed.

## Consequences

All current and future nested local credentials remain under one custody
boundary without a duplicate schema. The non-secret Plumber metadata is also
opaque on disk. Invalid or oversized JSON, unsafe paths, source drift, signing
drift, and unexpected archive contents stop installation or use.
