# 0057: Carry scoped temporary denial from iPhone

Status: accepted; extends ADR 0054 to the full iPhone Approval app.

## Decision

Offer the same two-minute Temporary Launcher Denial on iPhone when the Mac
would offer it: the same eligible Verified Launcher and gate have prompted
twice within thirty seconds, with a recognized operation threshold. Keep ordinary
Deny scoped to the current request. Do not add notification actions or phone
controls for canceling active rules.

Carry the offer in the existing authenticated request details, using one reserved
`Temporary Launcher Denial` section with exactly two nonempty rows, `Action` and
`Scope`. Older phones preserve those fields when reconstructing the request
digest and can continue to approve or deny normally. New phones recognize the
section and send the `temporaryDenial` outcome. Old Macs do not advertise it.
No weaker or alternate request digest is accepted.

Validate the request-bound response and explicit offer before accepting the new
outcome. Reject attempts to construct it from notification summaries. On the Mac,
claim the still-active, uncanceled prompt before creating the rule, with no
suspension between those operations. A resolved, canceled, superseded, or replayed
request cannot create or extend a rule. Use only the Mac's retained designated
requirement, gate, and threshold; never reconstruct enforcement scope from phone
strings. The existing continuous clock, denial checks, expiry, and menu-bar
cancellation remain authoritative.

Like ordinary Deny, this action needs neither a subscription nor biometric
approval. Phone Request History reports that the response was sent, not that
the Mac accepted it. No Secret or allow authority is released by this action.

## Consequences

This extends the human decision surface without moving the Local Execution
Boundary or adding persistent policy. The relay remains opaque transport and
needs no change. Older apps display the offer as request details without an
interactive temporary-denial action.
