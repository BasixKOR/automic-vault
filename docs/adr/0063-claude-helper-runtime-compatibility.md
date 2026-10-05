# ADR 0063: Claude Code helper runtime compatibility

Status: accepted
Amends: ADR 0009 for the built-in Claude Code association at Tool-specific gates

## Context

Issue #378 reports repeated Approvals after the default outside-bundle Claude
Code association introduced in ADR 0062. The association places Claude.app's
identity first but correctly retains the helper's runtime posture. Claude Code
disables library validation; rules enrolled against Claude.app commonly require
strict Hardened Runtime. The attributed helper therefore matches those rules
and fails their runtime check before another Launcher can supply authority.

Previously, a request could reach the strict Claude.app ancestor's rule without
applying that requirement to the Claude Code intermediary. That behavior does
not prove that every existing rule historically authorized Claude Code. This
decision deliberately accepts its known library-validation exception for the
reviewed association, including existing rules, rather than treating the helper
as strictly hardened or rewriting every Claude rule.

## Decision

At Tool-specific Secret Gates and Execution Gates, accept
`hardenedWithLibraryValidationDisabled` against a `hardened` rule only when the
Launcher identity came from the fully verified built-in association:

- helper `com.anthropic.claude-code`, Team `Q6L2SF6YDW`;
- parent `com.anthropic.claudefordesktop`, Team `Q6L2SF6YDW`.

The existing Developer ID, live and on-disk code identity, parent verification,
association settings, and applicable containment checks remain mandatory.
Only successful verification attaches the association to the Launcher identity.
An identifier, Team ID, filename, path, or request field alone cannot activate
the exception. Disabled associations and relocation opt-outs retain their effect.

Keep the helper's actual runtime posture. Missing Hardened Runtime, DYLD
environment-variable injection, disabled executable-page protection, and
debugger attachment do not qualify. Strict rules for Claude.app's own executable,
standalone Claude Code, and other helpers remain strict. Direct Access Rules
and Temporary Access Grants retain their existing runtime checks.

No policy migration is needed. Access Levels, Denial Thresholds, classification,
recording, and release checks continue to govern each operation. The exception
cannot turn Approval Required into automatic access or allow Unknown operations.
Retained Launcher Provenance follows its existing explicit opt-in and lifetime
rules; it carries the verified association with the original Launcher identity.

Show the runtime warning while adding Claude.app, in Verified Launcher Helpers
settings, and in the human Approval for enabling the association or its
outside-bundle permission. Explain that loaded third-party code can exercise
Claude's Tool-specific gate permissions, including existing rules. The existing
helper checkbox lets the user opt out across gates.

## Consequences

Existing Claude rules can authorize eligible Claude Code requests without
per-gate recreation. The accepted in-process trust boundary is weaker than
Claude.app's own runtime boundary: code loaded into Claude Code can exercise the
parent's configured authority. The warning describes this tradeoff; it does not
claim that all existing users previously acknowledged it.

Regression coverage must combine helper attribution with policy resolution at
both the GitHub CLI and SSH Agent gates, and reject failed verification,
opt-outs, wrong identities, unsafe runtime postures, and ordinary strict app
rules. General runtime eligibility must not gain a vendor exception.
