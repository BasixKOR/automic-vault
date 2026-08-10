# ADR 0010: Process-bound Secret proxy

## Context

Agentic Targets need to exercise credentials without placing reusable Secret
values in their environment, logs, history, or crash reports. An explicit HTTP
proxy can replace random placeholders at the last responsible moment, but a
loopback TCP proxy cannot securely determine which process opened a connection.
PID ownership inferred from port tables is racy and cannot be an authorization
boundary.

The network parser also handles attacker-controlled bytes. Giving that process
Keychain authority would combine two unnecessary privileges.

## Decision

`av proxy +NAME -- TARGET` creates a manually approved Proxy Session bound to
the complete command and the Gate Client's PID version and start time across its
`exec` transition. The launched Target receives one random Secret Reference per
Secret Name and a separate random Proxy Credential. Neither contains Secret
bytes.

A separately signed, sandboxed, Hardened Runtime helper owns the loopback proxy.
It has network client/server authority but no Keychain authority. It communicates
with the app over a private inherited channel. When an exact Secret Reference
appears in a public HTTP request, the helper submits redacted request metadata to
the app. The app verifies the live session, obtains a destination decision,
persists and verifies an Authorization Record, then returns only the Secret
values needed for that request.

Destination decisions are Deny, Allow Once, and Allow for Session. Session rules
bind exact Secret Names and canonical origin and exist only in memory. Every
Proxy Session start requires Approval; there is no Launcher blessing or durable
destination policy.

The helper fails closed for private or reserved destinations, DNS uncertainty,
invalid TLS, unsupported transformations, protocol upgrades, streaming bodies,
oversized messages, uninspectable responses, record failure, identity failure,
or loss of its private control channel.

## Consequences

- Raw Secrets do not enter the launched Target's environment or ordinary
  process memory through Automic Vault.
- PID identity safely limits session lifetime but does not authenticate TCP
  requests. Secret References and the Proxy Credential remain bearer values.
- Same-user malware that steals those bearer values can exercise existing
  Destination Rules until the session ends. Exact-origin rules prevent direct
  exfiltration to a different host but cannot prevent misuse of an approved
  service.
- Hardened Runtime is not required for the launched Target because manual use
  must support common interpreters. Missing or weakened protections are shown
  as an Approval warning because they increase inspection and injection risk.
- A Network Extension could add kernel-attributed flow identity later, but it
  would add installation, entitlement, and system-wide interception costs. It
  would reduce cross-process replay, not make an injectable Target trustworthy.
- Authorization History remains the single bounded local record. Proxy records
  share its existing global limit of 50.
