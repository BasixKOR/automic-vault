# ADR 0016: Agent-scoped Temporary Access Grants

Status: accepted

## Context

Agent-driven development often produces a short sequence of related write
requests. Approving each complete request preserves narrow authority but adds
friction. A global Full Access session would remove that friction by granting
far more authority than the user intended, including other gates, Launchers,
processes, and secret-disclosure operations.

Codex and Claude Code expose task or session UUIDs to their tool subprocesses.
Those values can narrow a grant to one agent task, but same-user software can
forge environment variables. They cannot establish identity or a security
boundary.

## Decision

Automic Vault supports an in-memory Temporary Access Grant with a fixed
ten-minute duration and Write Access. One grant binds exactly:

- one Tool-specific Authorization Gate;
- one Verified Launcher designated requirement;
- the Launcher's accepted Launcher Runtime Requirement; and
- one Agent Task Context: a Codex task UUID or Claude Code session UUID.

The service reads the context directly from the live XPC peer using a bounded
`KERN_PROCARGS2` query. It requires exactly one recognized variable containing
a canonical UUID. No client-controlled field is added to XPC. The context is a
forgeable narrowing label; the Verified Launcher remains the identity boundary.

A grant can begin only from an eligible live write-request Approval when the
user selects **Allow Write Access for 10 Minutes…**. The service then revalidates
the peer, context, Launcher, and runtime posture. It loads the current payload
and persists the required Authorization Record before starting the grant and
replying. Failure denies release and creates no grant. The decision does not
enter the transient Approval cache.

Future requests first complete ordinary Gate Client, Target, request, Secret,
gate, Launcher, and runtime verification. A valid Blessing continues to
authorize first. A grant may exceed a narrower Blessing only when the gate,
designated requirement, runtime posture, provider, task UUID, protection, and
operation all match exactly. Elevated Secret Application, Secret Disclosure,
Unknown operations, the Direct Secret Gate, Secret mutation operations, and
unverifiable Launchers are excluded.

The grant controller uses wall-clock and monotonic deadlines. Duplicate scopes
refresh to a newly confirmed generation. A generation-bound lease is held
through payload loading, Authorization Record persistence, retained-provenance
recording, and the XPC reply. Cancellation or expiry therefore cannot race an
in-progress release.

All grants are revoked on user-session inactivity, display sleep, update
installation, service stop, or app termination. Expiry and explicit End actions
revoke individual grants without authentication. A persistent aggregate strip
below the menu-bar item, mirrored menu actions, and an orange shield make active
escalation continuously visible.

Authorization History records future uses as policy-authorized by “Temporary
Access Grant — Write Access”. Exact task UUIDs remain memory-only and are not
written to Authorization History or telemetry.

## Consequences

Repeated recognized agent writes can proceed for a short period without
repeated Approval while remaining bound to the same verified software, gate,
runtime posture, and task label. A forged task UUID cannot cross the Verified
Launcher boundary, but code already running within that Launcher may share its
temporary authority; the task label does not prevent that.

The feature requires live process-environment inspection. macOS may withhold
arguments and environment for restricted processes; that uncertainty fails
closed and makes the prompt action unavailable.

The grant is deliberately unavailable from Settings, menus, CLI, URLs, or XPC;
has no duration picker; and is never persisted. A global Full Access session was
rejected because it would widen both scope and operation classes. Client-supplied
task identifiers were rejected because they would create an unnecessary trusted
input at the authorization boundary.
