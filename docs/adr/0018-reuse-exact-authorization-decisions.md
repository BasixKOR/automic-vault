# ADR 0018: Reuse exact Authorization Decisions across Approval transports

Status: accepted

## Context

An iPhone or Touch ID can carry a human Approval for one complete Authorization Request. Historically, transient reuse of an exact Authorization Decision has been independent of how the Approval was carried.

## Decision

The resulting Authorization Decision may receive the same memory-only transient reuse as a Mac-carried Approval. Reuse requires the same live process and complete request identity, expires after five minutes, and persists a fresh Authorization Record for every Secret Application; it reuses neither a phone response nor a biometric result. Operations that may receive long-lived AWS credentials remain excluded because their elevated credential exposure warrants fresh Approval.

## Consequences

- Fewer repeated prompts for identical requests within the existing transient reuse constraints.
- No reuse of phone responses or Local Authentication biometric results.
- AWS operations that may expose long-lived credentials still require fresh Approval.
