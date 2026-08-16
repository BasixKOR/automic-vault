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
The JavaScript resolver starts that bridge, which requests one exact Secret Name
through XPC. This is a Secret Disclosure because Varlock receives the raw Secret
Value. The approval service derives and binds the bridge's live parent,
selects the Secret Value from the bridge's physical working directory, and shows
the nearest Verified Launcher as the requester. The prompt and Authorization
Record identify the credential consumer as the Varlock plugin.

Varlock plugin requests always require a fresh human Approval. They cannot use
Authorization Policy, Direct Access Rules, transient Approval reuse, Blessings,
or Retained Launcher Provenance. The service revalidates the live bridge parent
and Verified Launcher after Approval and persists and verifies the Authorization
Record before it releases the selected Secret Value.

## Consequences

The plugin can resolve Secrets without the `av` CLI, and each Secret Use is
explicit. The extra signed process keeps the XPC trust boundary narrow. Initial
use is intentionally tedious; durable reviewed authority requires a later ADR
and implementation.
