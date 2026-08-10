# ADR 0010: Prohibit Ungated Secret Retrieval by Gate Clients

- Status: Accepted
- Date: 2026-08-10

## Context

The menu bar app owns Automic Vault's private Data Protection Keychain access
group. The standalone `av` Gate Client has no Keychain entitlement and uses XPC
for Secret operations.

An internal XPC `load` operation nevertheless allowed a correctly signed `av`
process to request any existing Secret Name and receive its raw value. The
operation was used for migration preflight and verification, but it did not
construct a complete Authorization Request, evaluate policy, obtain Approval,
or persist an Authorization Record. Code signing established the Gate Client's
identity; it did not establish authority for Secret Disclosure.

## Decision

The approval service will not expose a generic operation that returns an
existing Secret to a Gate Client.

Existing Secret bytes may leave custody only as an authorized Secret
Application or Secret Disclosure. The helper must evaluate the complete
request and persist and verify its Authorization Record before replying with
Secret bytes.

Secret mutation and migration checks that need to compare an incoming value
with an existing value run inside the menu bar app. They return success or a
non-secret error, never the stored value. Such mutations retain their explicit
human Approval and Authorization Record requirements.

Compatibility code does not justify a weaker path. The automatic migration of
the legacy `ARGOCD_CONFIG_YAML` Secret is retired instead of preserving an API
that releases the stored document to `av`.

## Consequences

- A signed copy of `av` cannot retrieve a Secret merely by naming it.
- Migration conflict checks are narrow, approved status operations rather than
  unrecorded Secret Disclosures.
- Old clients that send the removed `load` operation fail closed with an
  invalid-operation response.
- Users with the recent legacy Argo CD whole-config Secret must recover it
  through an explicitly approved Secret Use before running the current
  hardener.
- Release builds verify that `av` remains unentitled for Keychain access and
  that the menu bar app alone carries the exact private access group.
