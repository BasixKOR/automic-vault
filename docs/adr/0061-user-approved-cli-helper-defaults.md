# ADR 0061: User-approved CLI helper defaults

Status: accepted
Supersedes: built-in helper associations in ADR 0020 and the catalog/default-selection portions of ADR 0034

## Decision

Verified Launcher Helper associations are exclusively user-approved, exact,
path-bound records in the Data Protection Keychain. An absent, unreadable, or
malformed catalog enables no associations. Remove the built-in Codex and Claude
associations without converting implicit authority into user-approved records.
Existing user-approved associations and their disabled states remain valid.
Users whose CLI relied on a built-in association must add the parent app again
and approve the discovered helper; otherwise it may qualify under its own
standalone Launcher Identity, subject to ordinary policy and Approval.

When adding an app, preselect only discovered `codex` helpers under
`com.openai.codex`, with both Team IDs `2DC432GLL2`, and discovered
`com.anthropic.claude-code` helpers under `com.anthropic.claudefordesktop`, with
both Team IDs `Q6L2SF6YDW`. The ordinary signed, sealed, runtime-eligible discovery
checks still apply. Previously disabled exact associations and legacy `codex`/`claude-code` opt-outs
remain unselected.
All other helpers start unselected. Selection is presentation state and grants
no authority: the user may untick it, cancel, or approve the explicit cross-gate
authority expansion through the configured human Approval surface.

Runtime identity, relative-path, resource-seal, and outside-bundle permission
checks remain mandatory under their existing rules. No automatic migration or
new runtime vendor exception is introduced.
