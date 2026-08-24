# ADR 0026: Suspend Temporary Access Grant countdowns

Status: accepted

## Context

Temporary Access Grants currently expire after ten uninterrupted minutes. A
user may need to preserve the unused portion while temporarily stepping away
from an agent task. Leaving Write Access active while stopping its clock would
turn a bounded escalation into indefinite active authority and weaken the
grant's safety model.

This decision amends the fixed-duration details of
[ADR 0016](0016-agent-scoped-temporary-access-grants.md). Its scope, eligibility,
recording, lease, and lifecycle-revocation decisions remain unchanged.

## Decision

Each Temporary Access Grant starts with ten minutes of active countdown time.
The user may explicitly add ten minutes or suspend or resume its countdown from
the persistent strip. A suspended grant remains visible and memory-only but is
ineligible to authorize any request. Resuming restores eligibility with exactly
the frozen remaining duration.

Suspension freezes the lesser remaining duration from the wall-clock and
monotonic deadlines while holding the grant controller's existing lease lock.
Resumption creates new paired deadlines from that frozen duration. Suspension
and resumption therefore cannot race an in-progress release. An expired grant
cannot be suspended or resumed, and a newly confirmed duplicate scope replaces
any prior generation with a running ten-minute grant.

Adding time holds the same lease lock and adds ten minutes to both running
deadlines or to a suspended grant's frozen remainder. It does not restore Write
Access to a suspended grant and cannot revive an expired grant.

Session inactivity, display sleep, update installation, service stop, app
termination, and End continue to revoke suspended and running grants alike.
Suspension, resumption, and extension require an explicit user action but no new
Approval. Suspension removes authority, resumption restores only the remaining
confirmed scope, and extension changes only that same grant's active-time budget.

The strip shows the frozen remaining time and an explicit suspended state. The
countdown toggle is adjacent to End so immediate revocation remains continuously
available.

## Consequences

Pausing work no longer consumes the grant's active-time budget. The wall-clock
period and total authorizing time may exceed ten minutes only through explicit
user actions. Existing scope, runtime, task-context, operation, recording, and
release checks still apply to every use after resumption or extension.

Stopping only the visible clock while retaining active Write Access was rejected
because it would create unbounded authority behind temporary-access messaging.
