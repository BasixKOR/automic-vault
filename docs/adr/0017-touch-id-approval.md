# ADR 0017: Permit explicit Touch ID Approval on Mac

Status: accepted; transient reuse clarified by ADR 0018

## Context

iPhone Approval keeps allow controls away from an agent with Mac computer-use
access, but relay or phone unavailability can delay every human Approval. A
normal Mac button is not an acceptable fallback because the agent can drive it.
Touch ID can prove fresh physical user presence without granting pointer,
keyboard, password, passcode, or companion-device input the same authority.

Network failure is attacker-influenceable. It must not select a weaker approval
mode or silently change the user's authority model.

## Decision

Automic Vault provides opt-in Touch ID Approval per Mac.

- The opt-in is independent of relay state and may coexist with iPhone Approval.
- Enabling first requires the current human Approval surface, then a successful
  Touch ID evaluation on that Mac.
- The choice is stored in the app's Data Protection Keychain, not preferences.
- Each Approval uses a new Local Authentication context, biometric-only policy,
  and no biometric-result reuse interval.
- Password, passcode, Apple Watch, and pointer- or keyboard-driven allow actions
  cannot satisfy Touch ID Approval.
- The result authorizes only the exact pending Authorization Request or authority
  change that caused the prompt.
- If iPhone Approval is also enabled, the first valid phone or Touch ID result
  wins and the other pending transport is canceled.
- Disabling Touch ID Approval removes authority and requires no Approval.
- Unavailable or unenrolled Touch ID fails closed and does not disable iPhone
  Approval or restore an ordinary Mac allow action.

## Consequences

Users who enroll Touch ID Approval can continue working during a relay outage,
but they deliberately give up iPhone Approval's strict physical separation from
the Mac. The Mac must be active with awake displays for Touch ID Approval.

This amends ADR 0012's prohibition on every Mac-local allow surface. It does not
permit an outage-triggered fallback or any agent-drivable allow control.
