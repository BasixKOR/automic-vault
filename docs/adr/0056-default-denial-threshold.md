# 0056: Default Denial Threshold for Other Launchers

Status: accepted

The Authorization Gate default row needs the same allow/deny controls as named
Launcher rows. Store its optional Denial Threshold on the existing default
policy record and label the row Other Launchers (All Launchers with no specific
rules). The threshold applies when no Launcher-specific record matches the
attributed requirements, including empty attribution. It is a fallback, not a
gate-wide ceiling: named rows use their own thresholds, and every matching
explicit denial continues to win over all allow sources.

A denial-only named row inherits only the default allow level. Creating any
named row that weakens the fallback denial requires Approval, even if its allow
level is Approval Required. Compare the approved fallback under the policy lock
so a newly strengthened default cannot be bypassed by stale Approval. Reducing
default denial uses the same locked comparison as reducing a named denial.
Deleting a named rule restores the fallback and rechecks pending requests.
Existing records without a default threshold retain their behavior.
