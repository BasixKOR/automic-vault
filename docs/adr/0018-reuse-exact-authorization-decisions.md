# ADR 0018: Reuse exact Authorization Decisions across Approval transports

Status: accepted

An iPhone or Touch ID carries a human Approval for one complete Authorization
Request, but the resulting Authorization Decision may receive the same
memory-only transient reuse as a Mac-carried Approval. Reuse requires the same
live process and complete request identity, expires after five minutes, and
persists a fresh Authorization Record for every Secret Application; it reuses
neither a phone response nor a biometric result. Operations that may receive
long-lived AWS credentials remain excluded because their elevated credential
exposure warrants fresh Approval.
