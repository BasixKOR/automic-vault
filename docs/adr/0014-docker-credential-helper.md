# ADR 0014: Bind Docker registry credentials to live vendor-signed Targets

Status: accepted

## Context

Docker Desktop configures `credsStore` to invoke a credential helper. The
helper protects registry credentials at rest, but Docker's protocol returns a
usable password, personal access token, or identity token as plaintext JSON to
any process that invokes `get` with the registry address. The helper does not
authenticate the process or authorize its operation.

Docker Desktop ships its macOS CLI, Compose, and Buildx executables with
Docker's Developer ID Application identity and Hardened Runtime. Rebuilding
those Targets as Automic Vault Isotopes would replace a stronger upstream
distribution identity without removing the protocol's ambient-access flaw.
The Docker application bundle is user-writable, so its pathname alone cannot
establish Target integrity.

## Decision

The Docker Hardener installs an exact root-owned
`/usr/local/bin/docker-credential-av` launcher for the signed Automic Vault
CLI and changes Docker's default credential store to `av`. Every containing
directory through the filesystem root must also be a real, root-owned directory
protected from group/world writes; installation and detection fail closed if
that invariant is not met. It does not replace Docker's vendor CLI.

Each registry credential is stored as one opaque Secret. Its Secret Name is a
fixed prefix plus the SHA-256 digest of the exact registry address. The stored
value contains the address, username, and secret required by Docker's protocol.
The address remains visible in the Authorization Request and user-facing
explanation; the credential bytes do not leave Secret Custody for lookup or
verification.

For `get`, the helper submits the registry address to the Secret Gate. The menu
app derives the same Secret Name and obtains the helper's live parent from the
kernel. It accepts only explicitly supported Docker Desktop Targets at their
canonical bundle locations with Docker team `9BNSXJN65R`, the expected signing
identifier, a valid Developer ID signature, and an eligible Hardened Runtime.
The complete Authorization Request uses that live Target and its live
arguments. The parent process identity and code identity are revalidated after
any Approval and immediately before the Secret is loaded. The Authorization
Record is persisted and verified before the credential is returned.

`store` and `erase` use dedicated XPC operations. They accept only the same
signed Automic Vault helper beneath a verified Docker Target, validate the
registry-derived Secret Name, and use the existing approved Secret-mutation
path. They revalidate the Docker process before changing Secret Custody.

Hardening migrates credentials from the configured legacy helper without
printing them. It stores every credential with save-if-absent-or-equal
semantics before deleting any legacy entry. If deletion or the atomic Docker
configuration update fails, it restores deleted legacy entries and leaves the
old configuration in effect. Unsupported per-registry helper configurations
fail closed rather than being partially migrated.

The helper implements Docker's required `store`, `get`, and `erase` operations.
The optional `list` extension is not exposed because it would add registry and
username disclosure that Docker authentication does not require.

## Consequences

An arbitrary same-user process can still invoke the helper binary, but it
cannot retrieve, replace, or delete a Docker credential without a live eligible
Docker parent and an Authorization Decision for the actual Launcher, Target,
arguments, registry Secret Name, and process execution.

The authorized Docker Target necessarily receives the usable credential in
memory because the upstream protocol has no capability or operation-scoped
token interface. Automic Vault cannot make a compromised authorized Target keep
that credential confidential.

Docker Desktop updates remain vendor-managed. A changed Target continues only
if its live signing identity and runtime protections satisfy the allowlist.
Unsigned binaries, unexpected plugins, unsafe runtime exceptions, unknown
operations, and incomplete migrations fail closed. A code-signed Isotope
remains a fallback if Docker stops shipping an eligible required Target.
