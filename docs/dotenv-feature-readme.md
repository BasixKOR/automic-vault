# `av dotenv`

Runtime secret capabilities for applications that have outgrown ambient `.env`.

> [!NOTE]
> This is a README-style draft for the proposed `av dotenv` feature. The full
> design lives in [`dotenv-feature-spec.md`](./dotenv-feature-spec.md).

Traditional dotenv is convenient because every line of code can read every
secret once the process starts.

That was fine until agents learned to edit and run the code.

`av dotenv` keeps encrypted dotenv storage, but changes the runtime model:
applications request secrets explicitly through AV SDKs. Every request is
observable, approval-gated, logged, and tied to the callsite that asked for it.

## Quickstart

Start with the `.env` files you already have:

```sh
$ av dotenv ingest .
found .env
encrypted OPENAI_API_KEY
encrypted DATABASE_URL
stored project decryption key in Keychain
generated SDK migration prompt
# ^^ dotenv remains the migration format, not the runtime security boundary
```

Then update application code to ask for secrets instead of reading the process
environment:

```js
import { secret } from "@automic/av"

const apiKey = await secret("OPENAI_API_KEY")
```

First new access path:

```txt
OPENAI_API_KEY requested by:

src/lib/secrets.ts:12
  at src/llm/client.ts:48
  at src/routes/chat.ts:12

Approve this secret access?
```

Approve it once, or approve future matching requests. If the source path,
backtrace, executable, or project identity changes, AV asks again.

Annoying? A little. That is the point.

## What Gets Written

`av dotenv ingest` rewrites dotenv files with `dotenvx`-compatible encrypted
values:

```dotenv
# AUTOMIC VAULT MANAGED ENVIRONMENT
#
# Secrets are encrypted using dotenvx-compatible encryption.
#
# Secrets must be accessed through AV SDKs.

OPENAI_API_KEY="encrypted:ZXlKaGJHY2lPa..."
DATABASE_URL="encrypted:ZXlKaGJHY2lPa..."
STRIPE_SECRET_KEY="encrypted:ZXlKaGJHY2lPa..."
```

The encrypted files are intended to be committed. The project decryption keys
live in Keychain:

```txt
av.dotenv.project.<hash>.privatekey
```

Individual secrets are not stored separately in Keychain.

> [!WARNING]
> Plaintext secrets must not be written to disk: no plaintext dotenv files, no
> decrypted temp files, no plaintext caches, no logging, no shell history.

## The Migration is Agent-Shaped

Automic Vault assumes modern coding agents can modify repositories. So ingest
does not stop at "your `.env` is encrypted now, good luck."

It emits a migration prompt for agents:

```txt
Replace all environment-variable access:

  process.env.SECRET_NAME
  os.getenv(...)
  env::var(...)

with AV SDK usage.

Requirements:
- preserve existing behavior
- minimize unrelated edits
- do not log secrets
- do not serialize secrets
- do not expose secrets to frontend code
- prefer centralized secret access modules
```

You still review the patch. Obviously.

## Use a Secret Module

Prefer one boring place for secret access:

```js
// src/lib/secrets.ts

import { secret } from "@automic/av"

export async function openAIKey() {
  return await secret("OPENAI_API_KEY")
}
```

Then application code imports that helper.

Do not spray `secret("OPENAI_API_KEY")` through the codebase unless you enjoy
approval prompts that look like a stack trace fell down the stairs.

Centralized access gives AV stable callsites, cleaner logs, and better anomaly
detection.

## SDKs

Initial SDK targets:

- Node.js: `@automic/av`
- Python: `automic`
- Rust: `av`
- Go: planned
- Ruby: planned

Examples:

```python
from automic import secret

api_key = secret("OPENAI_API_KEY")
```

```rust
let api_key = av::secret("OPENAI_API_KEY").await?;
```

> [!IMPORTANT]
> This intentionally requires application changes. Transparent compatibility
> with unmodified dotenv applications is not the goal.

## What AV Watches

Every runtime request includes enough identity to decide whether it is the same
access path as before:

- secret name
- executable identity
- working directory
- normalized backtrace
- source fingerprints
- project identity

That lets unrelated code churn continue without retraining every approval. If
an agent changes the path that reads `STRIPE_SECRET_KEY`, that is related. AV
should interrupt.

Inspect what AV knows:

```sh
$ av dotenv info
managed files:
  .env

known secrets:
  OPENAI_API_KEY
  DATABASE_URL

observed callsites:
  OPENAI_API_KEY  src/lib/secrets.ts:12

sdk usage:
  node: detected
```

Revoke approvals when the baseline is no longer trusted:

```sh
$ av dotenv revoke OPENAI_API_KEY
revoked approvals for OPENAI_API_KEY
```

Or start over:

```sh
$ av dotenv revoke
revoked dotenv approvals for this project
```

## What This Replaces

The old model:

```txt
.env
  -> process environment
  -> any code in the process
  -> every dependency on that execution path
```

The AV model:

```txt
encrypted .env
  -> project key in Keychain
  -> SDK secret request
  -> captured backtrace
  -> approval engine
  -> plaintext returned in memory
  -> usage logged in the AV app
```

The core abstraction is no longer "environment variables." It is runtime secret
capabilities.

## What This Does Not Do

No, this does not prevent all secret exfiltration.

If approved application code receives a secret and then sends it to the wrong
place, AV cannot un-send it. The point is to make secret access explicit,
observable, and reviewable before it becomes normal behavior.

Also not goals:

- replacing production secret managers
- mandatory sandboxing
- invisible shell-wide injection
- supporting unmodified dotenv apps transparently
- preventing arbitrary malicious code execution

## Why Bother?

Because callsites are security information.

When `OPENAI_API_KEY` is always requested from `src/lib/secrets.ts`, that is a
baseline. When it starts getting requested from `src/debug/export.ts`, that is
a signal.

Development gets approval gates. The AV app gets an audit trail. Future
production monitoring can turn the same model into anomaly detection, alerts,
deployment blocking, and other paid sharp edges.

For the full draft:

> [`dotenv-feature-spec.md`](./dotenv-feature-spec.md)
