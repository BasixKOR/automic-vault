# ADR 0006: Direct Secret Access

- Status: Accepted
- Date: 2026-08-07

## Context

A Verified Launcher can submit a complete direct `av inject` request without a
Tool-specific Secret Gate. Approval can authorize that request once, but the
existing durable policies require either a Hardener-provided gate or an exact
Blessed Script. Some stable Developer ID-signed launchers need repeated access
to a small set of named Secrets while selecting commands dynamically.

Treating this as Secret Availability would conflate Keychain accessibility with
authorization. Attaching it to an unrelated Tool gate would misrepresent both
the Target and the granted authority.

## Decision

Automic Vault has one built-in Direct Secret Gate whose default Access Level is
Approval Required. A Direct Access Rule binds one exact Secret Name to one
Verified Launcher identity and permits Unconstrained Secret Application through
direct `av inject`.

Rules store the Launcher’s designated requirement, not its path or display name.
Every request revalidates the live code signature and required runtime
protections. Every requested Secret Name must have a matching rule for the same
Verified Launcher. Rules never apply to Secret Disclosure, Secret Name listing,
Secret mutation, or Tool-specific Gate Client requests.

The UI requires a fresh acknowledgement of the broader delegation before every
attempt to add a rule. Renaming or deleting a Secret revokes its Direct Access
Rules. Authorization Record persistence remains mandatory before Secret
Application.

## Consequences

- A permitted Launcher may choose any Target and arguments and can therefore
  cause the Secret to be disclosed after application.
- Tool-specific Hardening and Blessed Scripts remain the preferred alternatives
  because they constrain more of the Authorization Request.
- Invalid identity, unsafe runtime protection, malformed policy, incomplete
  Secret matching, or failed Authorization Record persistence cannot produce
  automic authorization.
- A Direct Access Rule does not propagate to another Secret Name, Launcher, or
  Gate Client.
