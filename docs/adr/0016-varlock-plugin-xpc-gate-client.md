# ADR 0016: Varlock plugin XPC Gate Client

## Status

Accepted

## Context

Varlock resolver plugins run inside Varlock and return resolved configuration
values to that process. Invoking `av inject` would misrepresent this flow as a
CLI Target launch and would add an unnecessary command boundary. Admitting an
arbitrary Node or Varlock process directly to the approval service would make
unrelated in-process code a trusted Gate Client.

## Decision

Automic Vault ships a dedicated, signed Varlock plugin bridge as a Gate Client.
The JavaScript package lives in the independent
[automic-vault/varlock-plugin](https://github.com/automic-vault/varlock-plugin)
repository. Before resolution, it requires every Automic Vault resolver to name
its Secret statically. At resolution it collects the complete active Secret Name
set and starts the bridge once. The bridge submits one multi-Secret
Authorization Request through XPC. This is a Secret Disclosure because Varlock
receives the raw Secret Values.

The package checks the signed bridge protocol before making a request. Each XPC
request carries the exact protocol version; the service rejects absent or
unsupported versions, and a successful response echoes the version for the
bridge to verify. Protocol version 1 first ships in Automic Vault 3.9.0. A
version mismatch fails closed.

The request includes a deterministic digest of the active Varlock schema
sources as descriptive request context. The digest cannot grant authority and
must not be reused as a Blessing or policy identity without independent trusted
verification.

The approval service derives and binds the bridge's live Varlock resolution
parent and that process's live application parent, selects Secret Values from
the bridge's physical working directory, and shows the nearest Verified
Launcher as the requester. The prompt and single Authorization Record identify
the credential consumer as the Varlock plugin and show every requested Secret
Name.

Each Varlock resolution run requires a fresh human Approval for its complete
multi-Secret request. It cannot use Authorization Policy, Direct Access Rules,
transient Approval reuse, Blessings, or Retained Launcher Provenance. The
service revalidates the live Varlock resolution process, application process,
and Verified Launcher after Approval and persists and verifies one
Authorization Record before releasing any selected Secret Value.

## Consequences

The plugin can resolve all Secrets needed by one Varlock run without the `av`
CLI and without presenting one prompt per Secret. Every Secret Name and selected
Value source remains explicit in one immutable Authorization Request. Rejecting
dynamic Secret Names ensures the set cannot grow after Approval. The extra
signed process keeps the XPC trust boundary narrow. Durable reviewed authority
still requires a later ADR and implementation. The independent package and app
can be released separately without silently accepting an incompatible wire
contract.
