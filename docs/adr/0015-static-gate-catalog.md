# ADR 0015: Separate Gate definitions from Hardener detection

Status: accepted

## Context

The approval service used `av hardeners --json` both to discover static Secret
Gate routes and to inspect every Tool's current Hardened State. Adding Docker
made that hidden coupling visible because Docker detection correctly asks
macOS to assess Docker Desktop's vendor distribution. Every unrelated
Authorization Request then waited for that diagnostic work.

Hardener detection describes the developer environment for the Dashboard and
Doctor. It does not establish authority for an Authorization Request. Runtime
authorization already verifies the live Gate Client, Target, Launcher,
complete request, policy, and required Authorization Record at the Local
Execution Boundary.

## Decision

The bundled `av` executable exposes an internal JSON catalog containing only
static Gate definitions. It is generated from the same Hardener modules but
does not call their detection functions. The approval service loads and
validates this catalog once at startup. An unavailable, empty, malformed, or
duplicate catalog prevents the service from starting.

Runtime matching uses every static definition. A request that exactly matches
a known Tool-specific Gate remains attached to that Gate even when current
Hardener diagnostic state is unavailable; it cannot fall through to the Direct
Secret Gate. Dynamic Hardener metadata remains the source for Dashboard and
Doctor status only.

## Consequences

Authorization Requests no longer execute unrelated filesystem, signature, or
platform-distribution diagnostics before presenting an Approval. Docker keeps
its installation-time and diagnostic vendor assessment, and Docker credential
requests keep their live Target verification before Secret Application.

The static and dynamic catalogs have separate code paths, so tests require
their Gate identifier sets to remain equal.
