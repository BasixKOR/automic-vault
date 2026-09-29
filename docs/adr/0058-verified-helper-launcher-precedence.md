# ADR 0058: Prefer the verified parent identity of a Launcher helper

Status: accepted

## Context

A Verified Launcher Helper may itself be packaged as a nested app. Codex CLI
inside ChatGPT is one example. Ordinary app discovery previously placed that
nested app before its verified parent association, and Approval presentation
chose the nearest enclosing app regardless of the selected Launcher Identity.

## Decision

After all existing helper association checks succeed, place the associated
parent identity before the helper's ordinary app identity for that process.
Keep both identities so explicit and temporary denials still cover either.
Existing policy resolution continues to consider explicit Launcher rules before
the gate default; when both identities have explicit rules, the associated
parent is considered first. Denial checks remain independent of that ordering.

Carry the verified app URL with each app Launcher Identity. Approval names,
icons, and Temporary Access Grant names use that app, not the nearest or
outermost enclosing bundle. A standalone Launcher retains its standalone
presentation. Presentation metadata grants no authority.

Disabled associations and failed validation do not promote the parent. All
live identity, signing, runtime, path, and seal checks remain as specified in
ADRs 0020, 0033, 0034, and 0055. Bundle containment alone never confers the
parent identity.

## Consequences

An enabled, verified Codex helper represents ChatGPT consistently in default
policy attribution and Approval presentation, including when packaged in
CodexCLI.app. Unassociated nested apps retain their own identities and labels.
No persisted policy or helper association is changed.
