# Automic Vault Product Positioning

Status: authoritative for user-facing messaging

Security claims and canonical terms defer to the [Domain Language](domain-language.md)
and [Architecture](architecture.md).

## Product promise

Automic Vault gives verified software bounded authority to apply developer
credentials to specific operations.

A retrieval-based secrets manager decides whether an identity may receive a
stored secret. Automic Vault authorizes the complete operation at the point
where software uses a developer credential. It considers the Verified Launcher,
Gate Client, Target, command, arguments, working directory, requested Secret
Names, and policy. Automic Vault asks the user when policy requires Approval.

## Short copy

**Headline:** Control how developer credentials are used.

**One sentence:** Automic Vault gives verified software bounded authority to
apply developer credentials to specific operations.

**Contrast:** Retrieval grants possession. Automic Vault authorizes Secret Use
at the Local Execution Boundary.

## Supporting claims

- Automic Vault protects credentials in custody and controls their application.
- Authorization binds the complete operation, not only an identity or Secret
  Name.
- Tool-specific Authorization Gates distinguish read, write, disclosure, and
  elevated credential use.
- Policy can authorize recognized operations. The user handles requests that
  require Approval.
- From an eligible agent write Approval, the user can use Touch ID to allow
  Write Access for that exact Verified Launcher, Tool-specific Authorization
  Gate, and agent task for ten minutes. A persistent strip shows the grant and
  offers an immediate End action throughout its lifetime.
- Existing developer commands continue to work above the security boundary.

## Claim boundaries

User-facing copy must preserve these limits:

- Code signing establishes software identity and integrity, not intent.
- After Secret Application, the Target controls the Secret in its memory,
  helpers, child processes, and output.
- Automic Vault does not contain root or kernel compromise, prevent arbitrary
  local destruction, or intercept every process execution.
- A Project Directory selects a Project Value. It does not establish identity
  or grant authority.
- A Codex task ID or Claude Code session ID is a forgeable narrowing label, not
  identity or a security boundary. The Verified Launcher remains the identity
  boundary for a Temporary Access Grant.
- Temporary Access Grants do not cover the Direct Secret Gate, Secret mutation,
  Elevated Secret Application, Secret Disclosure, or Unknown operations.
- Secret Disclosure remains available as an explicit, more powerful Secret Use.
- Execution control belongs to the same Developer Authority model even when an
  operation uses no Secret.

Do not claim that Automic Vault keeps every Secret invisible to its Target,
sandboxes the whole system, or makes verified software trustworthy.

## Architectural proof

- [ADR 0010](adr/0010-no-ungated-secret-retrieval.md) prohibits Gate Clients
  from retrieving a Secret merely by naming it.
- [Authorization Gates and Policies](adr/0002-authorization-gates-and-policies.md)
  bind policy to recognized operations and their characteristics.
- [Local Execution Boundary](adr/0001-local-execution-boundary.md) keeps
  enforcement on the Mac where the operation runs.
