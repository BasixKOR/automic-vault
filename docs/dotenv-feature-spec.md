# Automic Vault: Secure dotenv Environments

## Status

Draft Design Document

## Summary

`av dotenv` replaces plaintext `.env` workflows with encrypted, approval-gated secret injection while preserving compatibility with existing developer tooling and deployment systems.

The system is intentionally explicit.

Automic Vault does **not** attempt to infer developer intent, detect ecosystems automatically, or inject secrets ambiently into shells. Secrets are only made available to explicitly approved executables and only through `av dotenv run`.

The design prioritizes:

* zero-friction migration from plaintext `.env`
* compatibility with existing dotenv ecosystems
* compatibility with `dotenvx`
* explicit execution trust
* gradual security improvement
* future evolution toward capability-tracked secret access

---

# Goals

## Primary Goals

* eliminate plaintext `.env` files
* preserve existing development workflows
* support incremental adoption
* provide human approval gates for secret access
* provide visibility into secret usage
* support encrypted `.env` files committed to git
* maintain compatibility with `dotenvx`

## Non-Goals

* preventing all possible secret exfiltration
* mandatory sandboxing
* runtime taint tracking
* ecosystem autodetection
* transparent shell-wide secret injection
* replacing production secret managers

---

# Core Philosophy

Traditional `.env` systems provide:

```txt
ambient authority
```

Any process that can read the file or inherit the environment gains all secrets.

Automic Vault instead provides:

```txt
explicit capability grants
```

A secret is only made available:

* to approved executables
* for approved working directories
* after human approval
* at execution time

---

# Command Surface

## Ingest Existing dotenv Files

```sh
av dotenv ingest .
```

This command:

1. parses `.env*` files
2. encrypts secret values using dotenvx-compatible encryption
3. stores the private decryption key in Keychain
4. rewrites dotenv files with encrypted values
5. initializes execution policy metadata

---

## Run Commands With Secret Injection

```sh
av dotenv run -- node app.js
```

Examples:

```sh
av dotenv run -- npm run dev
av dotenv run -- python server.py
av dotenv run -- cargo run
```

---

## Show Environment Information

```sh
av dotenv info
```

Displays:

* managed dotenv files
* approved executables
* approval history
* encryption status
* last access times
* policy mode

---

## Approve Executables

```sh
av dotenv allow node
av dotenv allow python
av dotenv allow cargo
```

Explicit binary paths are also supported:

```sh
av dotenv allow /opt/homebrew/bin/node
```

---

## Remove Approval

```sh
av dotenv revoke node
```

---

## Reveal Secret Temporarily

```sh
av inject +OPENAI_API_KEY
```

This command requires human approval unless previously trusted.

---

# Supported Toolchains

The supported toolchain list is intentionally explicit because AV must install execution shims and wrappers.

Initial supported runtimes:

| Runtime | Notes                               |
| ------- | ----------------------------------- |
| node    | npm, pnpm, yarn, tsx, vite, next    |
| python  | python, uv, poetry                  |
| cargo   | rust tooling                        |
| go      | `go run`                            |
| ruby    | bundler, rails                      |
| php     | composer                            |
| java    | gradle, maven                       |
| docker  | compose + local container execution |

---

# Installation Consequences

`av dotenv` requires runtime interception.

This means AV installs shims/wrappers for supported executables.

Example:

```txt
/usr/local/bin/node
```

may become:

```txt
AV shim
→ real executable
```

or alternatively:

```txt
av dotenv run -- node app.js
```

may dynamically modify `PATH`.

The exact mechanism is implementation-defined but must preserve:

* predictable execution
* reversibility
* compatibility with developer tooling
* code signing integrity checks where possible

---

# Dotenv File Format

AV uses dotenvx-compatible encryption.

Example resulting `.env`:

```dotenv
# AUTOMIC VAULT MANAGED ENVIRONMENT
#
# Secrets are encrypted using dotenvx-compatible encryption.
#
# To inject secrets into a command:
#
#   av dotenv run -- node app.js
#
# Or request a temporary secret:
#
#   av inject +OPENAI_API_KEY
#
# Secret injection may require human approval.

OPENAI_API_KEY="encrypted:ZXlKaGJHY2lPa..."
DATABASE_URL="encrypted:ZXlKaGJHY2lPa..."
```

The resulting files are intended to be:

* safe to commit
* safe to share
* portable across environments

---

# Encryption Model

## Encryption Format

AV uses the same encryption format as `dotenvx`.

This provides:

* ecosystem compatibility
* production compatibility
* CI compatibility
* reduced cryptographic surface area
* easier migration between systems

---

## Key Storage

AV stores only the private decryption key in Keychain.

Example:

```txt
av.dotenv.project.<hash>.privatekey
```

Secrets themselves are never individually stored in Keychain.

---

# Security Invariants

## Secrets Must Never Exist Unencrypted On Disk

AV must never:

* write plaintext temp dotenv files
* cache decrypted secrets on disk
* log plaintext secrets
* store secrets in shell history

---

## Plaintext Exists Only In Memory

Secrets should only exist:

* transiently
* in process memory
* during approved execution

---

# Execution Model

## Secret Injection

Secrets are decrypted only during:

```sh
av dotenv run -- <command>
```

Execution flow:

```txt
command requested
↓
cwd resolved
↓
dotenv metadata loaded
↓
policy evaluated
↓
human approval if required
↓
secrets decrypted in memory
↓
child process spawned
↓
stdout/stderr monitored
```

---

# Approval Model

## Approval Modes

### ONCE

Approve a single execution.

---

### ALWAYS

Approve all future matching executions.

---

### IF UNCHANGED

Approve future executions only if execution identity remains unchanged.

This mode attempts to balance:

* usability
* agent iteration
* malware resistance

---

# Execution Identity

`IF UNCHANGED` fingerprints:

* executable hash
* argv
* working directory
* entrypoint hash
* requested secret set

Example:

```txt
sha256(
  executable +
  argv +
  cwd +
  entrypoint +
  secret_set
)
```

---

# Secret Leakage Detection

AV attempts best-effort leak reduction.

This is not considered complete exfiltration prevention.

---

## Stdout/Stderr Monitoring

AV scans process output for:

* exact secret matches
* partial secret matches
* high-entropy suspicious output

Possible responses:

* redact output
* warn user
* terminate process
* require re-approval

---

## Future Enhancements

Possible future enhancements include:

* filesystem write monitoring
* sandboxing
* syscall interception
* runtime secret resolution
* capability-tracked secret access
* `getenv()` interception

These are intentionally outside the scope of the initial implementation.

---

# Explicit Trust Model

AV intentionally avoids automatic ecosystem inference.

Users must explicitly approve trusted executables.

Example:

```sh
av dotenv allow node
av dotenv allow python
```

This preserves:

* auditability
* predictability
* comprehensible security boundaries

---

# Migration Story

## Existing Workflow

```sh
npm run dev
```

## AV Workflow

```sh
av dotenv run -- npm run dev
```

This is intentionally minimal friction.

---

# Compatibility

## Compatible With

* dotenv
* dotenvx
* existing runtime tooling
* git workflows
* CI/CD systems
* encrypted repo storage

## Not Compatible With

* ambient shell-wide env inheritance
* implicit secret injection
* transparent execution without approval

---

# Future Direction

The current design uses coarse-grained environment injection.

Future versions may evolve toward:

```txt
secret capability resolution
```

Examples:

* lazy secret resolution
* runtime `getenv()` interception
* per-callsite approvals
* code-path-aware secret access
* capability-scoped APIs

However, the initial release intentionally prioritizes:

* simplicity
* compatibility
* deployability
* developer adoption

over maximal isolation.
