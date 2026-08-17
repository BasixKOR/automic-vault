# Automic Vault Domain Language

Status: authoritative
Applies to: endorsed Automic Vault products, interfaces, and documentation

Automic Vault protects developer credentials at two boundaries: where they are stored and where they are used. Its domain is developer authority, with secrets and the application of those secrets at its center. Execution control, including Homebrew control, belongs to the same domain even when an operation uses no secret.

This document defines the language used across the Automic Vault ecosystem. Product copy should favor the user-facing terms. Architecture and implementation may use the precise terms where the distinction affects security.

## Product thesis

Automic Vault grants bounded Developer Authority over a complete operation. A
Launcher does not gain authority to retrieve a Secret merely by presenting an
identity or Secret Name. Each Authorization Decision covers one complete,
immutable Authorization Request, including the Verified Launcher, Gate Client,
Target, command, arguments, working directory, requested Secret Names, and
selected Secret Value sources.

Secret Application is the normal use of a Secret. Secret Disclosure is a
separate operation whose Authorization Request and Authorization Decision must
identify it explicitly. Execution Gates apply the same authority model to
controlled operations that use no Secret.

## Security model

The primary adversary is untrusted or compromised code running with the user's normal privileges: an agent, dependency, plugin, script, or supply-chain payload. Automic Vault builds on macOS code signing, Keychain, TCC, Hardened Runtime, and process identity, with the user as the final authority.

Automic Vault addresses two threat categories:

- **Unprotected Credential Access:** code can retrieve a credential through a plaintext file, environment variable, permissive Keychain item, credential-helper command, or similar mechanism.
- **Unauthorized Secret Application:** code causes a protected secret to be applied to an operation without authority from policy or the user.

Automic Vault does not claim to contain a root or kernel compromise, prevent arbitrary local destruction, or make a Target trustworthy after it receives a secret.

## Authority and gates

### Developer Authority

The authority to use developer credentials and perform controlled developer operations. Automic Vault helps the user delegate this authority to verified software with bounded policies.

### Authorization Gate

A boundary that decides whether an operation may proceed. Every gate owns one Authorization Policy.

- A **Secret Gate** controls the application or disclosure of protected secrets.
- An **Execution Gate** controls an operation without requiring a secret. Homebrew is the current example.

### Direct Secret Gate

The built-in Secret Gate for direct `av inject` requests that do not belong to a
Tool-specific Secret Gate. Its default Access Level is Approval Required.

A **Direct Access Rule** binds one exact Secret Name to one Verified Launcher and
grants Direct Access. It does not use patterns and does not authorize Secret
Disclosure, listing Secret Names, changing Secrets, or sibling Launchers. Because
the Launcher may choose any Target and arguments for each request, Direct Access
delegates more authority than a Tool-specific Secret Gate or Blessed Script.

### Local Execution Boundary

The point on the Mac where Automic Vault decides whether a Verified Launcher may apply a protected Secret or perform a gated operation through a Target. Secret storage, identity verification, policy evaluation, enforcement, and Authorization History remain on the Mac. A companion device may carry an Approval, but the Mac enforces the resulting Authorization Decision.

## Participants

### User

The human authority who chooses policy, grants Approval, stores Secrets, hardens Tools, and blesses scripts.

### Launcher

The app or executable at the root of the operation's verified launch chain, such as Terminal or Codex.

### Verified Launcher

A live Launcher whose code signature, designated requirement, and runtime protections meet the gate's eligibility rules. Code signing establishes identity and integrity, not intent. Failed verification prevents automic authorization.

Eligible Launchers enable Hardened Runtime. A gate may accept narrowly defined
compatibility exceptions while continuing to block runtime capabilities that
permit environment-driven code injection, disable executable-page protection,
or allow debugger attachment. A Launcher that disables library validation may
be eligible, but the UI must warn that third-party libraries and plug-ins can
run inside its process and inherit its authority.

### Launcher Bundle

An Automic Vault-generated macOS app bundle containing one fixed CLI payload
and a minimal launcher executable. The launcher starts the payload as its parent
process, and the bundle seals the payload. Enrollment binds the exact signed
bundle generation and final bundled payload digest. Automic Vault may ad-hoc
sign the bundle, or the user may select a Developer ID Application identity;
either form uses Hardened Runtime and remains bound to that exact generation.
The bundled payload has its own reserved child identity and is reverified when
live. It is not a separate Launcher Identity, but an exact live enrolled payload
may represent its Launcher Bundle Identity after its launcher exits, including
when the payload daemonizes.

Each Launcher Bundle exposes one Command through a root-owned command link. The
Command is how the user invokes the bundle; it is not Launcher Identity. The
installed bundle and command link are protected system artifacts, while the
original executable remains a separate unverified executable.

A Launcher Bundle is an artifact, not an Authorization Gate, Blessing, Isotope,
or Verified Launcher. When its enrolled identity, integrity, and runtime posture
are verified, its live launcher process or live enrolled payload representative
qualifies as the same Verified Launcher. The original unbundled executable has
a different identity and cannot match the Launcher Bundle's enrollment or
Launcher-specific rules.

Changing or re-signing any enrolled generation invalidates it. A bundle that
claims Automic Vault's Launcher Bundle identity but lacks matching enrollment
evidence is denied rather than considered as an ordinary Launcher.

### Launcher Runtime Requirement

The maximum Hardened Runtime exception profile accepted when a durable
Launcher rule is created. It is stored with the rule and checked against the
live Launcher on every request. Removing an accepted exception remains valid;
adding an exception beyond the stored requirement disables automic
authorization. Legacy rules that predate runtime requirements retain their
existing compatibility behavior.

### Agent Task Context

An ephemeral narrowing label for one recognized agent invocation. The initial
providers are a Codex task identified by a canonical UUID in
`CODEX_THREAD_ID` and a Claude Code session identified by a canonical UUID in
`CLAUDE_CODE_SESSION_ID`. Automic Vault accepts a context only when exactly one
recognized provider variable is present in the live process environment.

An Agent Task Context is forgeable by software running as the user. It is not
identity, authentication, or a security boundary and grants no authority by
itself. The Verified Launcher remains the identity boundary. Automic Vault
keeps the exact UUID in memory only and does not include it in Authorization
History or telemetry.

### Launcher Identity

The designated requirement stored when the user establishes trust and revalidated for each request. A path, display name, process identifier, or icon is metadata, not identity.

### Retained Launcher Provenance

Ephemeral evidence that Automic Vault recorded one exact live process execution
between a Gate Client and a Verified Launcher during a successful automic
authorization at one Authorization Gate. If the original parent chain later
disappears, detached-process access can recover the same Launcher attribution at
the same gate.

Retained Launcher Provenance is not an Approval, Authorization Policy, or new
Launcher Identity. Every later request is classified again under the gate's
current policy. It does not apply to another gate, process execution, user
session, or system boot, and uncertainty about the live process execution denies
reuse.

### Gate Client

The signed component that submits an Authorization Request, such as `av`, a patched `gh`, or the Homebrew stub.

### Target

The exact executable that performs the requested operation. For a Secret Gate, the Target is the intended consumer of the Secret Application. The designation limits the intended consumer but cannot prevent a compromised Target from leaking a secret after receipt.

### Tool and Command

A **Tool** is a developer-facing product or integration. A **Command** is a named executable entry point. A Tool may expose several Commands and Targets. A Hardener may manage more than one of them.

Use Launcher, Gate Client, and Target in security-sensitive prose. Avoid the ambiguous terms *caller* and *requester* except in external APIs, compatibility flags, or historical implementation names.

## Secrets

### Secret

A protected capability identified by one Secret Name. A Secret contains one or
more opaque, sensitive Values under Automic Vault's control.

### Secret Name

The identifier used to request a Secret. Use *Secret Name* instead of the overloaded word *key* in product language.

Authorization policy names the Secret, not an individual Value. A rule that
authorizes a Secret Name therefore covers every current and future Value of that
Secret Name.

### Secret Value

The opaque bytes selected when a Secret is used. A Secret may have one Global
Value and any number of Project Values.

### Global Value

The Secret Value used when no Project Value matches the Authorization Request's
working directory.

### Project Value

A Secret Value associated with one Project Directory. Project Values are
selection context, not an authorization boundary. The nearest matching Project
Directory is selected independently for each requested Secret Name.

### Project Directory

An existing directory represented by its canonical absolute path. The path is a
selector for a Project Value, not an identity or proof of authority. Moving the
directory does not move the Project Value; recreating a directory at the same
path makes that Project Value selectable again.

### Credential

One or more Secrets interpreted by a Tool or service. An AWS credential, for example, may contain an access key identifier and a secret access key.

### Secret Use

The umbrella term for Secret Application and Secret Disclosure.

### Secret Application

The release of a Secret to its designated Target for an authorized operation. This is the normal way Automic Vault uses secrets.

### Secret Disclosure

The intentional return of a raw Secret value to the Launcher, standard output, clipboard, or another general-purpose destination. `gh auth token` is a Secret Disclosure even though it has no remote side effect.

### Secret Availability

The Keychain availability chosen for a Secret and shared by all its Values:

- **When Unlocked:** available while the user's device is unlocked.
- **Available While Locked:** available after the first unlock following a restart.

Availability can prevent an authorized operation. It cannot authorize one. Human Approval requires an active user session and awake displays. An automically authorized operation may continue while locked only when every requested Secret is Available While Locked.

## Authorization

### Authorization Request

The complete, immutable description of one operation. It binds the Launcher,
Gate Client, Target, command, arguments, working directory, requested Secret
Names, selected Secret Value sources, relevant options, and process identity.

### Authorization Policy

The durable rules for one Authorization Gate. A policy contains:

- a default Access Level for every Verified Launcher without a specific rule;
- Launcher-specific rules chosen by the user.

An unverifiable Launcher does not receive the default Access Level. An unknown operation cannot be automically authorized.

### Policy Decision

The allow or deny result produced by applying an Authorization Policy to an Authorization Request.

### Approval

An explicit human decision to allow one Authorization Request. Approval never names a policy result.

### Authorization Decision

The final allow or deny result and its source. An allowed request is either **automically authorized** by policy or **approved** by the user.

### Temporary Access Grant

An in-memory, user-confirmed delegation of Write Access to one exact
Tool-specific Authorization Gate, Verified Launcher, accepted Launcher Runtime
Requirement, and Agent Task Context for ten minutes. It may automically
authorize recognized read and write operations at that scope. Elevated Secret
Application, Secret Disclosure, Unknown operations, the Direct Secret Gate, and
Secret mutation operations remain outside the grant.

A Temporary Access Grant can begin only when the user selects its explicit
action in an eligible live write-request Approval. It is not a
durable Authorization Policy or Blessing. Automic Vault shows every active
grant continuously with its successful-use count and last-use time, lets the
user end each grant immediately, and revokes all grants when the user session
becomes inactive, displays sleep, an update begins, or the service terminates.
Expiry uses both wall and monotonic clocks.

### Fail Closed

Uncertainty about policy or operation risk disables automic authorization and requires Approval. Uncertainty about identity, integrity, request completeness, Secret matching, selected Secret Value, or required Authorization Record persistence denies the request. An error must not fall back to another Secret Value or broader access.

## Operation Characteristics

The policy engine's target model describes an operation with a set of characteristics:

- **Read Only:** reads state without an intended side effect.
- **Homebrew Update:** refreshes Homebrew package definitions and local bookkeeping through `brew update`. It does not install, upgrade, reinstall, or remove software.
- **Local Write:** changes files or state owned by the current user.
- **System Write:** changes shared machine state, installed software, protected locations, or configuration outside the user's local project state.
- **Remote Write:** changes state in a remote service.
- **Elevated Secret Application:** applies a more powerful or reusable credential than the operation normally receives.
- **Unconstrained Secret Application:** applies a Secret through direct `av inject` to a Target and arguments selected by the Verified Launcher, without Tool-specific operation policy.
- **Secret Disclosure:** returns a raw Secret value to a general-purpose destination.
- **Unknown:** the Tool policy cannot establish the operation's characteristics.

An operation may have more than one characteristic. A deployment can perform System Write and Remote Write. An AWS read command that requires reusable keys can be Read Only and Elevated Secret Application. Policy must permit every characteristic. Unknown always prevents automic authorization.

Users should see the command and a concrete reason for an Approval request. System Write and Remote Write explain risk; they are not separate settings.

## Access Levels

An **Access Level** is a named policy preset for one Launcher at one Authorization Gate. Access Levels are not a universal ordinal ladder because gates expose only the presets that fit their operations.

| Access Level | Automic authorization |
| --- | --- |
| **Approval Required** | None. The user may approve a complete request once. |
| **Read Only** | Recognized Read Only operations. |
| **Read & Update** | Recognized Read Only and Homebrew Update operations. Available only at the Homebrew Execution Gate. |
| **Local Write** | Recognized Read Only and Local Write operations. |
| **Write Access** | Recognized read and write operations. Elevated Secret Application and Secret Disclosure still require Approval. |
| **Full Access** | Recognized operations, including Elevated Secret Application and Secret Disclosure. Unknown operations still require Approval. |
| **Direct Access** | Unconstrained Secret Application for exact Secret Names named by Direct Access Rules. Available only at the Direct Secret Gate. |

Every requested characteristic must fit the selected preset. Gates may omit presets that do not describe their operation set.

The Homebrew Execution Gate does not expose Read Only. Homebrew may update itself and its package metadata while running inspection commands, so Automic Vault treats Read Only and Homebrew Update as one indivisible level: Read & Update.

## Detection and hardening

### Detector

A read-only check for one supported Exposure or Hazard in the developer environment.

### Scan

A point-in-time run of the available Detectors. A clean Scan means no supported Detector found an issue. It does not mean the environment is secure.

### Finding

A concrete local condition reported by a Detector.

### Exposure

A usable path through which code running as the user can obtain a Secret or Credential.

### Hazard

A condition that increases the likelihood or impact of an Exposure.

### Compromise

Evidence that an unauthorized party obtained or used a Secret or Credential. Never infer Compromise from an Exposure or Hazard alone.

### Hardener and Hardening

A **Hardener** is a Tool-specific procedure that moves the Tool toward declared security invariants. **Hardening** applies that procedure through migration, configuration, a wrapper, or an Isotope.

### Hardened State

The current, verifiable invariants established by a Hardener. Hardened State does not mean fully safe.

### Doctor

A verifier for Automic Vault's intervention. Doctor checks declared invariants such as identity, ownership, permissions, dependencies, and command resolution. A Detector evaluates the developer environment; Doctor verifies the protection Automic Vault installed.

### Isotope

An Automic Vault-compatible build or wrapper of a third-party Tool. A Hardener
installs an Isotope through the Isotopes Homebrew tap when Homebrew is available,
or installs the same signed release directly and assumes responsibility for its
updates. An Isotope is not a Detector, Hardener, or Secret.

## Reviewed automation

### Script Declaration

The requested Secret Names, Target, injection options, and per-Gate capabilities declared by a script.

### Blessing

A durable record of human review bound to a script's canonical path, exact contents, and complete Script Declaration.

### Blessed Script

A script whose current file still matches its Blessing. Any content or declaration change invalidates the Blessing.

Execution normally uses a verified snapshot. If an interpreter cannot execute
that snapshot, the user may accept canonical-path execution while creating the
Blessing. Automic Vault warns during review and on every run because another
same-user process can change the file after verification and before the
interpreter opens it. The Blessing records the user's choice; an existing
Blessing cannot gain this exception during an upgrade.

### Capability

The maximum Access Level a Blessed Script may receive through one Authorization Gate.

### Launcher Endorsement

Permission for one Verified Launcher to execute an exact Blessing with automic authorization. Launcher Endorsement does not transfer to sibling Launchers or a changed script.

## Authorization History

### Authorization Record

A local record of an Authorization Request and its Authorization Decision. It includes the decision source, Launcher, Gate Client, Target, Secret Names, and operation. For an automically authorized Secret Use, it also records the Target's available Hardened Runtime posture at authorization time. This posture is diagnostic metadata, not Target identity evidence or an authorization input.

Automic Vault persists and verifies a record of allowed Secret Use before releasing a Secret. Failure to persist that required record denies the request. Records of denials and failures are best effort. Authorization History is bounded and local; it is not an append-only, tamper-proof, or complete forensic log.

## Least Authority

Durable trust binds one Authorization Gate to one Verified Launcher. A Direct
Access Rule additionally binds one exact Secret Name, but intentionally leaves
the Target and arguments under that Launcher’s control. A Blessing binds one
exact script and its capabilities. Approval binds one complete request and
process. When detached-process access is enabled, Retained Launcher Provenance
may preserve the same gate-and-Launcher attribution for one exact live process
execution after its parent chain disappears. Authority does not propagate to
sibling Launchers, changed scripts, another gate, a later process execution, or
a broader operation.

A Temporary Access Grant further narrows its Verified Launcher authority by an
exact Agent Task Context and accepted runtime posture. Because the task label is
forgeable, this narrows matching but does not strengthen Launcher identity.

## Zeroconf above the security boundary

Terminals, shells, IDEs, agents, harnesses, and projects keep using their existing commands. They need no Automic Vault plugin, policy file, or integration. Automic Vault discovers the local toolchain, applies secure defaults, and handles Tool-specific mediation beneath it. The user acts where intent matters: choosing Hardening, storing Credentials, approving requests, and trusting Launchers.

## Current implementation names

The following identifiers remain in storage or code for compatibility. New product language should use the canonical term.

| Legacy identifier or label | Canonical term |
| --- | --- |
| `No Access`, `noAccess` | Approval Required |
| `Read Only Access`, `readOnly` | Read Only; Read & Update at the Homebrew Execution Gate |
| `Read & Update Access`, `readOnlyAndUpdates` | Read & Update |
| `Local Write Access`, `readOnlyAndLocalWrites` | Local Write |
| `Trusted Access`, `fullExceptSecretDumps` | Write Access |
| `Full Access`, `fullIncludingSecretDumps` | Full Access |
| `secretDump` | Secret Disclosure or Elevated Secret Application, according to the operation |
| `mutating` | Local Write, System Write, Remote Write, or a combination |
| Secret Usage, access log, audit log | Authorization History |
| automatic approval | automic authorization |
| isotope key | Secret or Secret Name |
| caller | Launcher, Gate Client, or Target, according to the role |

Persisted raw values and external compatibility flags may retain legacy spelling. Their display names and documentation follow this language.
