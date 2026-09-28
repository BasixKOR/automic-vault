# ADR 0002: Model Authorization with Gates and Policies

- Status: Accepted
- Date: 2026-08-06

## Context

Automic Vault controls both Secret Application and sensitive execution that may use no secret. A model named only for secrets cannot describe Homebrew. A single risk label also loses important distinctions: a command may write local and remote state, disclose a secret without writing anything, or need a reusable credential for a read operation.

Users still need a policy interface they can learn without composing security traits by hand.

## Decision

Use **Authorization Gate** as the general boundary, with **Secret Gate** and **Execution Gate** as specializations.

Each gate owns one Authorization Policy with an explicit default Access Level and optional Verified Launcher rules. An unverifiable Launcher does not receive the default. Unknown operations cannot be automically authorized.

The policy engine's target model assigns a set of Operation Characteristics: Read Only, Homebrew Update, Local Write, System Write, Remote Write, Elevated Secret Application, Secret Disclosure, and Unknown. Homebrew Update means `brew update`: package-definition and bookkeeping changes without installing, upgrading, reinstalling, or removing software. Policy must permit every characteristic.

The user-facing policy presents named Access Levels:

- Approval Required
- Read Only
- Read & Update, only for the Homebrew Execution Gate
- Local Write, where the gate supports it
- Write Access
- Full Access

System Write and Remote Write appear as reasons for Approval, not separate user settings. Elevated Secret Application and Secret Disclosure remain outside Write Access. Full Access includes them for recognized operations. Unknown still requires Approval.

The Homebrew Execution Gate omits Read Only. Homebrew may update itself and its package metadata while running inspection commands, so its three presets are Approval Required, Read & Update, and Full Access.

### Gate-specific labels (2026-09-28)

Authorization Gate controls name the capability added by the top Secret Gate preset: **Secret Disclosure**, including Write Access. AWS instead displays **Elevated Secret Application**, reflecting its reusable-credential path. Execution Gates retain **Full Access**; signing and authentication gates retain their specialized labels. Use these labels consistently for allow ceilings and Denial Thresholds.

This is a presentation change. Persisted values, operation classification, effective authority, threshold ordering, Unknown handling, and Blessed Script syntax remain unchanged. Full Access remains the generic preset name when no particular gate supplies context.

## Compatibility

The current implementation persists legacy Access Level raw values and enforces one legacy request classification. Existing Homebrew Read Only rules intentionally broaden to Read & Update because the old level could not prevent incidental Homebrew updates. Other display-name changes do not change stored grants. A future characteristic-set migration must review the Tool catalog, preserve or narrow every stored grant, and test unknown and combined operations before replacing the legacy classifier.

## Consequences

- Homebrew fits the model as an Execution Gate.
- Homebrew's default can authorize metadata updates without implying authority to upgrade installed software.
- The architecture can describe combined effects without expanding the settings UI.
- Product copy can state why a request needs Approval.
- New Tool policies must classify conservatively and fail closed on unknown commands.
- A policy migration cannot infer broader authority from a legacy label.
