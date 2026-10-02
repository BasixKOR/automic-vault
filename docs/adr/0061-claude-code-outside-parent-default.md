# ADR 0061: Default Claude Code association outside Claude.app

Status: accepted

## Context

Claude Desktop installs its Claude Code runtime outside Claude.app under its
Application Support directory. Inspection of Claude.app 2.16120.0 confirms a
versioned runtime manager that downloads and checks a manifest-pinned archive,
then extracts claude.app/Contents/MacOS/claude. Our built-in association therefore
fails to cover this normal installation without the relocation exception.
See issue #372 and [Anthropic's desktop setup](https://code.claude.com/docs/en/desktop-quickstart).

## Decision

Enable outside-bundle support by default only for the built-in association from
com.anthropic.claude-code to com.anthropic.claudefordesktop, both signed by
Q6L2SF6YDW. Reuse ADR 0055 verification without relaxing its remaining checks:
exact Developer ID identities, eligible live runtime protections, live/on-disk
identity equality, and the installed verified parent app remain mandatory.
Inside-bundle helpers retain resource-seal checks. Paths and cache markers confer
no authority.

Version 2 helper settings apply this default to new and legacy configurations,
including legacy empty relocation sets. Existing disabled associations remain
disabled. Version 2 persists an explicit relocation opt-out across reloads.
Malformed configuration continues to fail closed. Other helpers remain opt-in;
re-enabling an exception after opt-out retains the existing human Approval.

## Consequences

This deliberately broadens existing Claude Launcher-specific rules across all
Authorization Gates to matching eligible Claude Code executions outside the
parent bundle. It includes separately installed copies and other signed versions,
not just desktop-downloaded runtimes. It does not prove installation provenance
or pin the helper to Claude.app's bundled manifest. Code signing proves identity
and integrity, not intent. Users may disable relocation or the association and
may retain separate Claude Code rules. No Access Level or gate policy changes.
