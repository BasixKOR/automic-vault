# ADR 0001: Keep the Execution Boundary Local

- Status: Accepted
- Date: 2026-08-06

## Context

Automic Vault can use a companion device for an Approval response, but the protected Secrets, live process identities, Targets, and developer environment exist on the Mac. Splitting enforcement across a remote service or companion device would add network authority and weaken the binding between a decision and the live operation.

## Decision

Automic Vault enforces authorization at the Local Execution Boundary on the Mac.

The Mac:

- stores Secrets and policy;
- verifies the Launcher, Gate Client, Target, and request integrity;
- evaluates the Authorization Policy;
- persists Authorization History;
- releases Secrets and permits gated execution.

A companion device may present an Authorization Request and return the user's Approval or denial. The response binds the complete request. The Mac validates it and makes the final Authorization Decision.

Developer tools, terminals, IDEs, and agent harnesses keep their normal command interfaces. Automic Vault mediates supported Tools beneath them.

## Consequences

- Loss of network connectivity does not move Secret custody or policy evaluation off the Mac.
- A companion compromise cannot release a Secret without the Mac accepting a response for a live matching request.
- Remote Approval needs a cryptographic binding to the complete request.
- PATH wrappers provide a scoped mediation boundary, not general process containment.
- The system remains zeroconf above the boundary, while setup and user decisions remain where security intent requires them.
