# Automic Vault Secret Capabilities

## Status

Draft Design Document

---

# Summary

Automic Vault replaces ambient environment-variable secrets with observable, approval-gated runtime secret capabilities.

Traditional `.env` systems expose secrets globally to all executing code within a process. In the era of autonomous agents this model is no longer sufficient.

Automic Vault instead requires applications to explicitly request secrets through AV SDKs.

Every secret request:

* is observable
* is logged
* captures a backtrace
* is approval-gated
* becomes part of a behavioral security model

The system is designed around the assumption that modern AI agents are capable of rewriting application codebases automatically. As such, introducing SDK/library requirements is considered acceptable when it produces materially better security and observability.

---

# Core Philosophy

Traditional dotenv systems provide:

```txt id="rjlvlf"
ambient authority
```

Once a process starts, any executing code can access all secrets.

Automic Vault instead provides:

```txt id="om4n3k"
observable capability grants
```

Secrets are not inherited passively.

Secrets are requested actively by executing code.

This distinction is foundational.

---

# Goals

## Primary Goals

* eliminate plaintext secrets
* eliminate ambient secret access
* provide runtime observability for secret usage
* approval-gate new secret access paths
* make agent-driven code modification safer
* maintain dotenvx-compatible encrypted storage
* support behavioral anomaly detection
* support future production monitoring products
* provide a low-friction migration path from `.env`

---

# Non-Goals

* preventing all secret exfiltration
* replacing production secret managers
* mandatory sandboxing
* transparent compatibility with unmodified dotenv applications
* invisible shell-wide injection
* preventing malicious code execution

---

# High-Level Architecture

The system consists of four major components:

| Component              | Responsibility                                |
| ---------------------- | --------------------------------------------- |
| Encrypted Secret Store | dotenvx-compatible encrypted storage          |
| AV SDKs                | runtime secret capability requests            |
| Approval Engine        | evaluates new secret access paths             |
| AV App                 | observability, logging, approvals, monitoring |

---

# CLI Surface

## Ingest Existing dotenv Files

```sh id="jv1brs"
av dotenv ingest .
```

This command:

1. parses `.env*` files
2. encrypts values using dotenvx-compatible encryption
3. stores project decryption keys in Keychain
4. rewrites dotenv files with encrypted values
5. initializes AV metadata
6. generates migration prompts for coding agents

---

## Display Project Information

```sh id="qg2h6c"
av dotenv info
```

Displays:

* managed dotenv files
* known secrets
* approval history
* observed callsites
* encryption status
* SDK usage status
* last secret access timestamps

---

## Revoke Approvals

```sh id="qjlwmn"
av dotenv revoke
```

or:

```sh id="afzr4g"
av dotenv revoke OPENAI_API_KEY
```

---

# Example Resulting dotenv File

```dotenv id="s1lgqd"
# AUTOMIC VAULT MANAGED ENVIRONMENT
#
# Secrets are encrypted using dotenvx-compatible encryption.
#
# Secrets must be accessed through AV SDKs.
#
# JavaScript:
#   import { secret } from "@automic/av"
#
# Python:
#   from automic import secret
#
# Rust:
#   av::secret(...)

OPENAI_API_KEY="encrypted:ZXlKaGJHY2lPa..."
DATABASE_URL="encrypted:ZXlKaGJHY2lPa..."
STRIPE_SECRET_KEY="encrypted:ZXlKaGJHY2lPa..."
```

These files are intended to be:

* safe to commit
* safe to share
* portable between environments

---

# Encryption Model

## Encryption Format

AV uses dotenvx-compatible encryption.

Reasons:

* ecosystem compatibility
* CI/CD compatibility
* reduced cryptographic surface area
* production interoperability

---

## Key Storage

AV stores only project decryption keys in Keychain.

Example:

```txt id="ew3htd"
av.dotenv.project.<hash>.privatekey
```

Individual secrets are not stored separately in Keychain.

---

# Security Invariants

## Plaintext Secrets Must Never Exist On Disk

AV must never:

* write plaintext dotenv files
* cache plaintext secrets
* log plaintext secrets
* store secrets in shell history

---

## Secrets Exist In Plaintext Only In Memory

Secrets should only exist transiently during approved runtime access.

---

# Runtime SDKs

Automic Vault requires explicit runtime SDK usage.

This is intentional.

---

# Initial SDK Support

| Platform | Package       |
| -------- | ------------- |
| Node.js  | `@automic/av` |
| Python   | `automic`     |
| Rust     | `av` crate    |
| Go       | planned       |
| Ruby     | planned       |

---

# JavaScript Example

```js id="wjlwm9"
import { secret } from "@automic/av"

const apiKey = await secret("OPENAI_API_KEY")
```

---

# Python Example

```python id="omce1z"
from automic import secret

api_key = secret("OPENAI_API_KEY")
```

---

# Rust Example

```rust id="jstbuh"
let api_key = av::secret("OPENAI_API_KEY").await?;
```

---

# Runtime Secret Resolution

Secret access occurs dynamically at runtime.

Example flow:

```txt id="o0i9fx"
application requests secret
↓
AV SDK captures runtime metadata
↓
approval engine evaluates request
↓
human approval if required
↓
secret decrypted in memory
↓
secret returned to caller
↓
usage logged in AV app
```

Secrets are never globally injected into process environments.

---

# Backtrace Requirement

Every secret request must include a runtime backtrace.

This is a core architectural requirement.

Example:

```txt id="rypr0u"
OPENAI_API_KEY requested by:

src/lib/secrets.ts:12
└── src/llm/client.ts:48
    └── src/routes/chat.ts:12
```

This allows AV to reason about:

* where secrets are used
* how secrets are used
* whether usage patterns changed

---

# Secret Capability Identity

Secret access approvals are tied to:

* secret name
* executable identity
* working directory
* normalized backtrace
* source fingerprints

Not merely process identity.

---

# Approval Model

## Approve Once

Approve a single runtime request.

---

## Approve Always

Approve future matching requests indefinitely.

---

# Re-Approval Triggers

Re-approval is required if:

* backtrace changes
* executable changes
* source fingerprint changes
* project identity changes
* module graph changes

This intentionally allows agents to modify unrelated application code without constantly requiring re-approval.

Only secret-related execution paths matter.

---

# Logging and Observability

All secret usage is logged in the AV app.

Example timeline:

```txt id="8kprcx"
10:14 AM
OPENAI_API_KEY requested
src/lib/secrets.ts

10:16 AM
DATABASE_URL requested
src/db/index.ts

10:18 AM
NEW SECRET ACCESS PATH DETECTED
Approval required
```

The AV app becomes an operational visibility layer for authority usage.

---

# Behavioral Security Model

Automic Vault treats secret usage as a behavioral signal.

Known-good development behavior becomes a baseline.

New or unexpected secret usage patterns become anomalies.

---

# Production Monitoring (Paid)

Future paid functionality will support production anomaly detection.

Example:

```txt id="3mk3pv"
Production service accessed STRIPE_SECRET_KEY
from previously unseen callsite:

src/debug/export.ts
```

Possible actions:

* alerts
* Slack notifications
* audit logging
* deployment blocking
* approval escalation

This system is intended to detect:

* compromised agents
* malicious code paths
* accidental secret misuse
* unexpected production behavior

---

# Agent Migration Support

Automic Vault assumes modern coding agents are capable of adapting repositories automatically.

The migration experience is therefore designed around agent-assisted code rewriting.

---

# Migration Flow

```sh id="kx4b3m"
av dotenv ingest .
```

Produces:

* encrypted dotenv files
* initialized AV metadata
* generated migration prompts
* detected secret inventory
* SDK installation instructions

---

# Generated Agent Prompt

After ingest, AV emits a prompt intended for coding agents such as Codex, Claude Code, OpenAI agents, etc.

Example:

```txt id="t8vhx5"
This repository now uses Automic Vault runtime
secret capabilities.

Replace all environment-variable access:

  process.env.SECRET_NAME
  os.getenv(...)
  env::var(...)

with AV SDK usage.

JavaScript:
  import { secret } from "@automic/av"

Python:
  from automic import secret

Rust:
  av::secret(...)

Requirements:

- preserve existing behavior
- preserve async semantics
- minimize unrelated edits
- preserve tests
- do not log secrets
- do not serialize secrets
- do not expose secrets to frontend code
- prefer centralized secret access modules
- avoid repeated secret fetches
- cache secrets appropriately

Detected secrets:

- OPENAI_API_KEY
- DATABASE_URL
- STRIPE_SECRET_KEY
```

---

# Recommended Secret Access Pattern

AV strongly recommends centralized secret-access layers.

Preferred:

```js id="q83v4h"
// src/lib/secrets.ts

import { secret } from "@automic/av"

export async function openAIKey() {
  return await secret("OPENAI_API_KEY")
}
```

Discouraged:

```js id="mjlwmf"
await secret("OPENAI_API_KEY")
```

scattered throughout the codebase.

Centralized access improves:

* auditability
* approval stability
* migration quality
* observability clarity

---

# Relationship To dotenv

dotenv ingestion is considered migration tooling.

The core abstraction is no longer:

```txt id="m77a6g"
environment variables
```

The core abstraction is:

```txt id="x47f0m"
runtime secret capabilities
```

---

# Explicit Design Tradeoff

Automic Vault intentionally imposes a runtime library burden.

This is considered acceptable because:

* modern agents can rewrite applications automatically
* explicit secret access enables observability
* explicit secret access enables behavioral security
* explicit secret access enables approval gating

The resulting security model is materially stronger than traditional dotenv systems.

---

# Future Directions

Possible future enhancements include:

* capability-scoped secrets
* secret intent metadata
* temporary secret leases
* filesystem exfiltration monitoring
* runtime taint tracking
* sandbox integration
* production policy enforcement
* agent-specific secret policies
* semantic anomaly detection
* callsite reputation systems
* secret usage risk scoring

These are intentionally outside the scope of the initial implementation.
