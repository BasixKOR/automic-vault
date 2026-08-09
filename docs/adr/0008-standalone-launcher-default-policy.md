# ADR 0008: Apply Gate Defaults to Standalone Verified Launchers

- Status: Accepted
- Date: 2026-08-09

## Context

Authorization Policies define a default Access Level for every Verified
Launcher without a launcher-specific rule. Standalone Developer ID-signed
executables were initially required to have an exact enrolled rule, so a gate's
default did not apply when the verified launch chain contained only a standalone
launcher. This contradicted the policy model and made an **All Apps** default
silently behave differently for signed CLI launchers.

Applying the default without checking standalone runtime protections would also
be incorrect. It would bypass the Hardened Runtime eligibility required when a
user adds an explicit rule for the same launcher class.

## Decision

A gate's default Access Level applies to an eligible standalone Verified
Launcher when no launcher-specific rule matches. Standalone eligibility requires
the existing live Developer ID identity checks and Hardened Runtime protections.
An unsigned, ad-hoc signed, invalid, or runtime-ineligible standalone executable
receives no default authority.

Launcher-specific rules continue to match the exact designated requirement and
take precedence across the verified launch chain. A path, display name, or Team
ID alone does not establish a match. Existing launcher-specific rules retain
their stored runtime-requirement flag for compatibility; new rules continue to
require Hardened Runtime. Unknown operations continue to require Approval, and
every allowed Secret Use still requires a persisted, verified Authorization
Record before release.

## Consequences

- Existing gate defaults now govern app bundles and eligible standalone
  executables consistently.
- A broader existing default may automically authorize recognized operations
  from a hardened standalone launcher that previously required Approval.
- Explicit Approval Required rules still narrow the default for one exact
  Launcher Identity.
- Failure to verify standalone identity or runtime protection continues to fail
  closed rather than falling back to broader access.
