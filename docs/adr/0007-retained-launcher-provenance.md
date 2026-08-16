# ADR 0007: Retain Launcher Provenance for Detached Processes

- Status: Accepted
- Date: 2026-08-07

## Context

Long-lived developer harnesses can outlive the Verified Launcher that started
them. macOS may reparent a server under a terminal or app to PID 1 after its
parent exits. Later Gate Clients remain descendants of the same server process,
but ordinary process ancestry no longer proves which Verified Launcher supplied
policy.

Remembering a prior Approval would be incorrect: Approval authorizes one complete
Authorization Request. Remembering PID and start time alone would also be
unsafe because `exec` preserves both while replacing the executable image.

## Decision

Automic Vault may retain ephemeral Launcher provenance after a successful
automic authorization. One record binds:

- one Authorization Gate;
- the exact Verified Launcher that supplied authority;
- one signed intermediary process execution identified by PID, PID version,
  start time, user and audit session, and live code identity.

Automic Vault creates a provenance record only after it persists the required
Authorization Record. Human Approval never creates Retained Launcher
Provenance. Automic Vault keeps provenance in memory, revalidates it before every
use, and removes it when the process execution or menu bar helper exits.

When normal ancestry cannot supply authority, a matching record may restore the
original Launcher attribution only for its recorded gate. Automic Vault then
classifies the complete current Authorization Request and applies the gate's
current policy. It does not reuse the earlier Authorization Decision. Policy
changes and revocation therefore take effect immediately, Unknown still requires
Approval, and every allowed Secret Use still requires a new Authorization
Record.

A global **Keep Launcher Access for Detached Processes** setting controls the
behavior and defaults to off. Automic Vault may keep non-authorizing shadow
records while the setting is off so an Approval window can explain when the
setting would have permitted the current request.

## Consequences

- Portal, herdr, and similar harnesses use one generic process-provenance model;
  no product name or executable basename becomes an identity.
- A process cannot inherit provenance after PID reuse, `exec`, code replacement,
  another user session, menu bar helper restart, or use at another gate.
- Enabling the setting extends a Launcher's gate-specific authority beyond the
  lifetime of its visible parent chain. A process can intentionally detach and
  retain that attribution until its exact execution exits.
- Ad-hoc signed processes remain more exposed to same-user code injection than
  Hardened Runtime launchers. The UI recommends creating a Launcher Bundle for
  recurring harness use.
- If supported macOS APIs cannot establish the exact process execution or live
  code identity, Automic Vault fails closed and does not retain or reuse it.
