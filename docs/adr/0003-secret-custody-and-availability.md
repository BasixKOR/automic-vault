# ADR 0003: Separate Secret Custody, Availability, and Authorization

- Status: Accepted
- Date: 2026-08-06

## Context

A stored Secret has two independent questions: can the operating system return its bytes in the current lock state, and may this operation receive it? Treating availability as policy could let a storage choice grant authority. Treating policy as availability could make unattended work fail after the user chose a scoped grant.

## Decision

Store Secrets in the macOS Data Protection Keychain and model Secret Availability apart from authorization.

The availability choices are:

- When Unlocked
- Available While Locked, after the first unlock following a restart

Availability may deny an authorized request. It never grants one. Authorization still requires a valid Authorization Decision for the complete request.

Human Approval requires an active user session and awake displays. Requests waiting for a human decision are denied when that condition stops. An automically authorized request may proceed while locked only when all requested Secrets are Available While Locked.

Automic Vault records and verifies an allowed Secret Use before releasing bytes. Record persistence failure denies release. Denial and failure records remain best effort.

## Consequences

- Users can make an explicit tradeoff for unattended work without broadening Launcher authority.
- Policy tests must cover lock-state availability as a separate prerequisite.
- Product language must not imply that Available While Locked means broadly accessible.
- Authorization History is useful local history, not a tamper-proof forensic log.
