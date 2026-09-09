# ADR 0045: Transport Homebrew Agent Task Context through the signed stub

Status: accepted

## Context

The Homebrew Gate Client runs setuid as the dedicated `automic` user. macOS
withholds its process environment from the approval service's `KERN_PROCARGS2`
query. Consequently a recognized agent's write request cannot offer or match a
Temporary Access Grant, even when its Verified Launcher is eligible. A
byte-identical signed stub running as the invoking user exposes the same task
variable successfully.

Reading a parent's environment is not equivalent: an agent may set a task UUID
only for a particular child execution. Removing setuid would break Homebrew's
protected ownership model. Neither is an appropriate fix.

## Decision

Amend ADR 0016 with one transport exception for the Homebrew Execution Gate.
The signed brew stub reads its own environment before submitting its single
Authorization Request and may send `brew_CODEX_THREAD_ID` or
`brew_CLAUDE_CODE_SESSION_ID` over its existing authenticated XPC connection.
It sends no context when multiple recognized variables (including duplicate
entries) are present, or when the selected value is not a canonical 36-byte
UUID: hexadecimal digits with hyphens at the UUID positions. Uppercase and
lowercase hex are accepted, matching the service's validation. These fields
are never command-line inputs or Launcher identity claims.

The approval service accepts these fields only from the verified brew stub for
an `authorize` request with no Secret Names, the `brew` Tool, and the matched
Homebrew Execution Gate. Each present field must be a 36-byte XPC string; the
existing Agent Task Context parser requires exactly one provider with a
canonical UUID. Missing, malformed, oversized or ambiguous context leaves the
ten-minute option and grant matching unavailable. Other gates continue to use
live process-environment inspection; SSH remains excluded.

The immutable context belongs to that one request and its live Gate Client.
Initial matching, queued matching and grant activation use the same resolver.
The ordinary live process, signature, Verified Launcher, runtime posture,
operation, empty capability ceiling, expiry and Authorization Record checks
remain mandatory before allowing execution. No context is inferred from a
parent or persisted in Authorization History or telemetry.

## Consequences

A task label remains forgeable by same-user software and grants no authority by
itself. The user still explicitly grants Write Access scoped to a Verified
Launcher, runtime posture, Homebrew gate and exact task label. The exception
changes transport, not the identity boundary or allowed operation classes.

The stub version advances so doctor detects older installations for refresh.
An old stub omits these fields and continues to require ordinary Approval when
policy does not authorize the request. An old app ignores the new fields and
continues to withhold temporary grants. Both the updated app and stub are
required to restore the feature.
