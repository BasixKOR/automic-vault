# ADR 0065: Explicit overrides of descendant Launcher rules

Status: accepted

## Context

A Launcher can deliberately start a workload that exercises its gate permissions.
Unsigned intermediaries and shells need not supply their own Launcher rule.
However, Terminal must not silently override a nearer agent's explicit rule.
Some harnesses, such as bb, should be able to act as the policy source for
Launchers they start, after a user explicitly chooses that scope.

## Decision

Add “Override descendant Launcher rules” to each explicit Launcher rule in the
Authorization Gate editor. Default off, including legacy records. Use the nearest
explicit rule normally; the outermost enabled ancestor override takes precedence.
Require live original-parent execution evidence between the nearer Launcher and
the overriding ancestor. Do not treat helper aliases of one process or an appended
Retained Launcher Provenance identity as proof of ancestry.

Keep every matching explicit Deny as a veto. Keep runtime requirements on both
the selected rule and overridden explicit rules. Unknown still requires Approval
unless denied. Denial-only rows cannot override. Direct Access, Blessings, and
Temporary Access Grants retain their own authority semantics.

Stage checkbox edits in the existing Review Changes flow. Both directions require
Approval: disabling a restrictive ancestor override can reveal a broader child
rule. Compare the full reviewed Launcher rule under the policy lock before
persisting either change. Missing fields default off; malformed values fail
policy decoding. Preserve the setting through allow and denial edits.

Record a warning in Authorization History when the selected override allows the
current operation but an overridden explicit rule would require Approval.
SSH ancestry discovery collects verified ancestors beyond the nearest Launcher;
it never crosses an unavailable original-parent link to obtain override authority.

## Consequences

Existing rules keep their precedence until the user approves an override.
An override can broaden or narrow access and is scoped to one Gate. Shells with
no explicit rule do not create warning noise. This does not amend the installed-app
helper association in ADR 0062 or claim to fix issue #382 by itself.
