# Automic Vault · Operational Danger Classification

## Overview

Automic Vault does not classify software as “safe” or “unsafe”.

Instead, software is classified according to:

> How catastrophic compromise or misuse of this software could become.

This model assumes:
- any binary can eventually become compromised
- any writable executable is suspect
- any interpreter can synthesize arbitrary future behavior
- trust is temporary unless continuously validated

Thus:
- installation trust
- operational capability
- mutability
- containment
- provenance
- runtime behavior

all contribute to a package’s effective danger level.

---

# Core Principles

## Safe Installation ≠ Safe Execution

A package being allowed to install normally does not imply:
- unrestricted execution
- unrestricted capabilities
- unrestricted device access
- unrestricted network access

Example:

```sh
av install ffmpeg
````

may be allowed normally.

But:

```sh
ffmpeg -f avfoundation -i ":0" out.mp4
```

would trigger approval gates because it invokes:

* microphone access
* camera access
* screen capture

---

## Capability Boundaries Matter More Than Intent

Automic Vault focuses on:

* observable capabilities
* operational containment
* provenance validation
* human approval

rather than attempting to infer “intent”.

The system assumes:

* intent inference can fail
* binaries can be compromised
* agents can become adversarial

Therefore:

* effects matter more than claimed purpose
* runtime operations matter more than package descriptions

---

# Operational Danger Levels

## Green · Appliance

Low amplification if compromised.

### Characteristics

* deterministic behavior
* narrow operational scope
* no plugin ecosystem
* no arbitrary code execution
* minimal or no networking
* difficult to repurpose

### Examples

* jq
* ripgrep
* fd
* sed
* tree
* pngquant

### Compromise Impact

Generally localized.

Compromise usually does not:

* create autonomous behavior
* enable persistence
* synthesize arbitrary new capabilities

---

## Blue · Tool

Powerful but operationally bounded.

### Characteristics

* broad file access
* optional networking
* transforms/imports/exports data
* may interact with devices
* limited scripting or macro support
* no generalized runtime evaluation

### Examples

* ffmpeg
* git
* curl
* rsync
* sqlite
* imagemagick

### Compromise Impact

Can:

* exfiltrate data
* surveil users
* manipulate files

But generally cannot:

* evolve dynamically
* synthesize arbitrary future behavior
* easily bypass containment

### Typical AV Policy

* install normally
* runtime capability gates
* monitor sensitive invocations

---

## Yellow · Runtime

Generalized behavior synthesis engines.

### Characteristics

* interpreters
* JIT runtimes
* arbitrary eval
* plugin ecosystems
* package managers
* subprocess spawning
* dynamic module loading

### Examples

* Node.js
* Python
* bash
* zsh
* Ruby
* Lua
* Bun
* Deno

### Compromise Impact

A compromised runtime becomes:

```txt
generalized arbitrary behavior generation
```

These systems can:

* generate new tools dynamically
* bypass assumptions
* evolve behavior over time
* execute unbounded future logic

### Typical AV Policy

* vaulted installation
* stronger containment
* extensive approval gating
* continuous integrity verification

---

## Orange · Infrastructure

System reshaping tools.

### Characteristics

* package management
* virtualization
* daemon orchestration
* environment mutation
* supply chain manipulation
* privilege mediation

### Examples

* Docker
* Homebrew
* npm
* pip
* cargo
* launchctl
* virtualization systems

### Compromise Impact

Can:

* alter trust boundaries
* poison environments
* establish persistence
* mutate software ecosystems
* affect large portions of the system

### Typical AV Policy

* high ceremony
* sandboxing
* verbose approval UX
* privileged operation monitoring

---

## Red · Escape / Surveillance / Offensive

Direct bypass and offensive capability tooling.

### Characteristics

* process injection
* debugging
* packet capture
* accessibility abuse
* exploit tooling
* credential interception
* kernel interaction

### Examples

* lldb
* gdb
* tcpdump
* mitmproxy
* metasploit
* aircrack-ng

### Compromise Impact

Can:

* bypass AV controls
* extract secrets
* surveil users
* escalate privileges
* enable lateral movement

### Typical AV Policy

* strongest warnings
* explicit human approval
* enhanced telemetry
* aggressive containment
* continuous revalidation

---

# Integrity Confidence Levels

Danger level alone is insufficient.

Every executable also has an integrity state.

---

## Verified

* trusted signature
* trusted provenance
* immutable installation root
* signature chain intact

Operational trust is highest.

---

## Drifted

* binary hash changed
* contents modified unexpectedly
* signature mismatch detected

Operational trust degraded.

---

## Ad-hoc Signed

Executable has been re-signed locally.

Example:

```sh
codesign -s -
```

This indicates:

* provenance chain collapse
* local mutation
* unverifiable trust lineage

Automic Vault should treat this as high risk.

---

## Mutable

Executable writable by:

* user
* runtime
* external process

Trust continuously degrades over time.

---

## Unknown

* provenance unavailable
* signature unverifiable
* integrity state indeterminate

Requires conservative handling.

---

# Trust Model

Automic Vault is not:

* antivirus
* malware detection
* intent classification

Automic Vault is:

```txt
a continuously validated operational trust graph
```

Trust emerges from:

* provenance
* signatures
* immutable install roots
* operational capabilities
* runtime behavior
* containment
* human approvals
* continuous verification

---

# Core Philosophy

The objective is not:

```txt
prevent all bad behavior
```

The objective is:

```txt
prevent software from escaping its declared capability envelope
without detection, containment, or human approval
```

---

# Design Implications

## Recommended Defaults

### Green / Blue

* install normally
* capability-based runtime gating
* lightweight monitoring

### Yellow

* vaulted installation
* aggressive integrity monitoring
* execution approvals

### Orange

* explicit operational warnings
* containment required
* infrastructure mutation telemetry

### Red

* maximum ceremony
* strongest approvals
* continuous revalidation
* sandbox enforcement

---

# Final Principle

No software is permanently trustworthy.

Trust must be:

* earned
* constrained
* observable
* continuously revalidated

```
