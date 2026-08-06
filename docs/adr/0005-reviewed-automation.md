# ADR 0005: Bind Reviewed Automation to Exact Scripts

- Status: Accepted
- Date: 2026-08-06

## Context

Automation can compress many Approval prompts into one reviewed unit. Trusting a path, mutable file, project, or whole agent harness would allow unreviewed content to inherit that authority.

## Decision

A Script Declaration names requested Secrets, the Target and injection options, and per-Gate capabilities.

A Blessing records human review of the script's canonical path, exact contents, and complete Script Declaration. A script is a Blessed Script only while the file matches that record. Execution uses a verified snapshot so edits cannot race the authorization decision.

A Capability limits the maximum Access Level available through one Authorization Gate. A Launcher Endorsement permits one Verified Launcher to execute the exact Blessing with automic authorization. Blessing and endorsement are separate choices.

Authority does not transfer to sibling Launchers, another path, a changed script, or a capability absent from the declaration.

## Consequences

- Editing a script or its declaration invalidates the Blessing.
- An unendorsed Launcher still needs Approval to run the Blessed Script.
- The UI must show the exact script identity, declaration, capabilities, and endorsed Launcher before durable trust is recorded.
- Experimental automation integrations remain outside this model until the project endorses them.
