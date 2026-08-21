# Automic Vault [![Knock Knock](https://outclaw.dev/badge.svg)](https://outclaw.dev/automic-vault/automic-vault)

> Your secrets manager should know what the secrets *do*.

Automic Vault is a macOS secrets manager for developer tools and agents. It
moves supported credentials out of plaintext files and checks the complete
operation before applying a credential.

## Authorization Covers the Operation

Most secrets managers check an identity and Secret Name before returning the
stored value.

Automic Vault checks the Verified Launcher, Tool, Target, command, arguments,
working directory, Secret Names, and selected Secret Value sources. Policy
evaluates the complete Authorization Request on the Mac where it will run.

With Read Only access, one GitHub token produces three decisions:

```text
gh issue list     → automically authorized
gh issue create   → Approval required
gh auth token     → Secret Disclosure; Approval required
```

Automic Vault controls the handoff. The Target controls the Secret after
receiving it.

## Quickstart

Download the [latest release], or install with Homebrew:

```sh
brew install --cask automic-vault/isotopes/automic-vault
open /Applications/Automic\ Vault.app
```

Audit the machine:

```sh
av scan
```

Automic Vault reports supported Exposures and Hazards. Each Finding includes a
mitigation. Harden a supported Tool, then verify the result:

```sh
av harden gh
av doctor gh
```

[latest release]: https://github.com/automic-vault/automic-vault/releases/latest

> [!IMPORTANT]
> Automic Vault is not associated or affiliated with any cryptocurrency or
> “token”.

## What It Protects

The primary adversary is untrusted or compromised code already running with
your normal user privileges: an agent, dependency, plugin, script, or
supply-chain payload.

Automic Vault builds on macOS code signing, the Data Protection Keychain, TCC,
Hardened Runtime, and live process identity. Its features include:

- continuous detection for over 100 developer-tool configurations that expose
  credentials or create related hazards;
- Secret Custody outside plaintext dotfiles, environment setup, permissive
  Keychain items, and ambient credential-helper commands;
- Tool-specific Authorization Gates that understand recognized read, write,
  disclosure, elevated, and unknown operations;
- Authorization Policies scoped to each Verified Launcher;
- Project Values selected by physical working directory under stable Secret
  Names;
- Proxy Sessions that keep Secret Values out of the Target and apply them only
  to approved outbound HTTP/S requests;
- Blessed Scripts bound to reviewed contents and declared capabilities;
- in-memory Temporary Access Grants for eligible agent tasks;
- optional iPhone Approval for carrying every human Approval away from an
  enrolled Mac;
- local Authorization History for allowed and denied requests.

Your terminals, IDEs, agents, and projects keep their normal commands. Agents
need no Automic Vault plugin, and repositories need no policy file.

## Authorization Gates

Hardening adds an Authorization Gate for the Tool. The gate classifies each
request and applies its default Access Level or a rule for the requesting
Verified Launcher.

<img src="./docs/img/authorization-gate-v4.jpg" alt="Automic Vault Authorization Gate" style="width: 589px; height: auto" />

Access Levels include:

1. **Approval Required:** Automic Vault asks for Approval on every operation.
2. **Read Only:** Automic Vault automically authorizes recognized reads.
3. **Read & Update:** Automic Vault automically authorizes Homebrew reads and
   `brew update`.
4. **Local Write:** Automic Vault automically authorizes recognized reads and
   local-only writes where the Tool supports this distinction.
5. **Write Access:** Automic Vault automically authorizes recognized reads and
   writes. Disclosure and elevated application still need Approval.
6. **Full Access:** Automic Vault may automically authorize recognized sensitive
   operations. Unknown operations still need Approval.

Automic Vault offers human Approval while the user session and displays are
active. It aborts open approvals when either becomes inactive. Policy-approved
requests may continue, subject to each Secret's **Available While Locked**
setting.

Code signing proves identity and integrity, not intent. You decide which
Verified Launchers receive authority. A failed identity or runtime check blocks
automic authorization.

## Move Every Human Approval to iPhone

iPhone Approval is optional and enabled per Mac. Once enabled, every human
Approval for that Mac moves to eligible iPhones on the same iCloud Keychain
account. The Mac shows the request and its cancellation state with no local
allow action.

The iPhone app requires an iPhone Approval subscription: $4.99 monthly, or
$39.99 annually with a 14-day free trial. Family Sharing is not supported.

The Mac remains the Local Execution Boundary. It verifies the complete
Authorization Request, rejects stale or mismatched responses, persists the
Authorization Record, and enforces the final decision. The iPhone never
receives Secret Values or Authorization History.

To enroll:

1. Sign in to the same iCloud account on the Mac and iPhone, with iCloud
   Keychain enabled.
2. Open Automic Vault on the iPhone, tap **Enable iPhone Approval**, and allow
   notifications.
3. On the Mac, open **Settings → iPhone Approval** and click
   **Enable iPhone Approval**.

Routine requests can offer **Approve Once** in an authenticated notification.
Requests with Unknown operation risk, Secret Disclosure, Unconstrained Secret
Application, or a security warning require review in the full app. Face ID or
Touch ID is optional and configured on each iPhone. When enabled, a passcode,
Apple Watch, or companion Mac cannot substitute for biometrics on that phone.

Every iPhone enabled on the account can carry Approvals. The initial release
has no per-device pairing or revocation; emergency recovery invalidates the
whole account enrollment.

> [!WARNING]
> iPhone Mirroring and **Show on Mac** can put Approval controls back onto a Mac
> when biometric protection is off. Disable those features wherever an agent
> can control the Mac, or require Face ID or Touch ID on every eligible iPhone.

If no phone or relay is available, the request waits until its Gate Client
cancels. Automic Vault never restores a Mac allow button. Emergency recovery
from the Mac requires system authentication, cancels pending requests, rotates
the iCloud key, and invalidates every enrolled iPhone and Mac on the account.

## Project Values Under Stable Names

A Secret Name can have one Global Value and multiple Project Values:

```sh
av save API_TOKEN
av save --project-directory=. API_TOKEN
```

When `av inject` requests `API_TOKEN`, Automic Vault selects the value for the
nearest physical project directory at or above the working directory. If no
Project Value matches, it selects the Global Value. The same `API_TOKEN` name
works across projects.

The Project Directory selects a value and grants no authority. The same
name-based policy covers all Values of that Secret. A read failure for the
selected Value ends the request without trying another value.

For dotenvx, store the decryption key in Automic Vault and remove `.env.keys`:

```sh
av save --project-directory=. DOTENV_PRIVATE_KEY
av inject +DOTENV_PRIVATE_KEY -- dotenvx run -- npm test
```

dotenvx decrypts the project file only after Automic Vault authorizes applying
its project-selected key to that operation.

## Secret Proxy

For an application that reads a credential from its environment:

```js
const response = await fetch('https://api.example.com/me', {
  headers: { Authorization: `Bearer ${process.env.API_TOKEN}` },
});
```

store the Secret, then launch the application through the proxy:

```sh
av save API_TOKEN
av proxy +API_TOKEN -- node --use-env-proxy app.js
```

`API_TOKEN` is a random, session-specific Secret Reference inside the launched
Target. When that exact reference appears in an outbound request, the signed,
sandboxed proxy asks whether to apply the Secret to that destination. Approval
is required for the Proxy Session and each new destination; **Allow for
Session** remembers only that origin and Secret Name until the Target exits.

Node's built-in `fetch` needs Node 24 or newer and `--use-env-proxy`. Other
clients must respect the supplied proxy and scoped CA environment variables.
See [Secret Proxy](docs/secret-proxy.md) for compatibility and the exact
security boundary.

### Keep Secrets out of `.env`

Leave non-secret project configuration in `.env`, but omit the Secret:

```dotenv
API_ORIGIN=https://api.example.com
# API_TOKEN comes from Automic Vault
```

Store a Project Value, then load the rest of `.env` normally:

```sh
av save --project-directory=. API_TOKEN
av proxy +API_TOKEN -- node --use-env-proxy --env-file=.env app.js
```

The working directory selects the Project Value. The loader must preserve an
existing `API_TOKEN`; an override option would replace the Secret Reference and
the proxy could not apply the Secret. Never write the reference into `.env`—it
is random and valid only for one Proxy Session.

## Varlock

Install [Varlock](https://varlock.dev) and the published
[Automic Vault plugin](https://github.com/automic-vault/varlock-plugin):

```sh
npm install --save-dev varlock @automic-vault/varlock-plugin
av save API_TOKEN
```

Declare the Secret in `.env.schema`:

```dotenv
# @plugin(@automic-vault/varlock-plugin)
# @disableProcessEnvInjection
# ---
# @sensitive @required
API_TOKEN=av()
```

Load Varlock, then read the Secret through `ENV` rather than `process.env`:

```js
import 'varlock/auto-load';
import { ENV } from 'varlock/env';

const response = await fetch('https://api.example.com/me', {
  headers: { Authorization: `Bearer ${ENV.API_TOKEN}` },
});
```

The resolver infers the Automic Vault Secret Name from `API_TOKEN`. Use
`API_TOKEN=av(OTHER_SECRET_NAME)` when they differ. Secret Names must
be static so the Approval shows the complete set before any Secret Value is
released.

> [!IMPORTANT]
> Requires Automic Vault 3.9.0 or newer. Varlock currently requires one Approval
> on every run for the complete active Secret set. Automic Authorization and
> Blessings are not supported for the Varlock plugin yet.

### Keep the Application on Varlock Placeholders

Varlock also has its own credential proxy. It composes with the Automic Vault
resolver, so `.env.schema` can own the destination rule while Automic Vault
keeps custody of the Secret:

```dotenv
# @plugin(@automic-vault/varlock-plugin)
# @disableProcessEnvInjection
# ---
# @sensitive @required
# @proxy(domain="api.example.com")
API_TOKEN=av()
```

```sh
varlock proxy rules
varlock proxy run -- node app.js
```

Automic Vault approves the complete active Secret set and releases it to the
live Varlock resolution process. Varlock gives the application a placeholder
and applies the real value only to requests matching the schema rule. This is a
[Varlock credential proxy](https://varlock.dev/guides/proxy/) session, not an
Automic Vault Proxy Session; don't nest it inside `av proxy` because both need
to own the process proxy and CA environment. Varlock's proxy is currently a
preview, so read its limitations before relying on its boundary.

## Scripts and Agent Tasks

### Blessed Scripts

Automic Vault binds a Blessing to the script's canonical path, contents, Script
Declaration, and requested capabilities:

```sh
av bless --endorse-launcher ./scripts/deploy
```

```sh
#!/usr/local/bin/av inject +DEPLOY_TOKEN -- /bin/bash
# --- automic-vault
# capabilities:
#   gh: read-only
#   aws: write
# ---
```

Editing the script or declaration invalidates the Blessing. A Launcher
Endorsement lets one Verified Launcher automically authorize that Blessing. Use
Blessed Scripts for reviewed work that exits, and Tool Authorization Gates for
long-running processes.

<img src="./docs/img/blessed-script.png" alt="Automic Vault Blessed Script review" style="width: 589px; height: auto" />

### Temporary Access for Agent Tasks

When an eligible Codex task or Claude Code session requests a write, the
Approval window can offer **Allow Write Access for 10 Minutes…**. Automic Vault
stores the Temporary Access Grant in memory and binds it to the Verified
Launcher, Tool-specific gate, runtime posture, and current agent task.

<img src="./docs/img/temporary-write-access.png" alt="Automic Vault temporary write access controls" style="width: 589px; height: auto" />

The persistent strip shows active grants and lets you end them early. Automic
Vault also revokes them when the user session becomes inactive, displays sleep,
an update begins, or the app stops.

The Verified Launcher remains the identity boundary. The task identifier is a
forgeable label that narrows the grant. Temporary Access Grants exclude the
Direct Secret Gate, Secret mutations, Elevated Secret Application, Secret
Disclosure, and unknown operations.

## Verified Launchers for Unsigned CLIs

Automic Vault rejects unsigned and arbitrary ad-hoc-signed executables as
Verified Launchers. For a regular single-file Mach-O CLI, it can create a
Launcher Bundle from a snapshot of that executable:

- Automic Vault signs the launcher and payload with Hardened Runtime.
- The installer makes the bundle and `/usr/local/bin` command link root-owned.
- Automic Vault revalidates the live code identity, generation, payload digest,
  nested signatures, and runtime posture on each authorization.
- A modification or new signature hard-denies the request.

Launcher Bundles establish identity and integrity for the packaged code. They
cannot establish publisher trust or make the CLI safe. Automic Vault supports
neither scripts nor directory-shaped tools. See [Signed CLI Launchers] for the
requirements and update behavior.

[Signed CLI Launchers]: docs/signed-cli-launchers.md

## AWS and Docker Credential Handoffs

AWS hardening removes the default long-lived key pair from
`~/.aws/credentials` and installs a native credential helper:

```sh
av harden aws
aws sts get-caller-identity
```

Each invocation registers its arguments, profile, process identity, and config.
The helper gives normal commands short-lived STS credentials. Automic Vault
shows an Elevated Secret Application warning when an operation requires the
original reusable keys. It installs and verifies AWS's signed CLI under
`/opt/av/aws`; credential providers outside the supported profile model fail
closed.

Docker hardening migrates registry credentials out of ambient helper access:

```sh
av harden docker
docker pull registry.example/acme/image
```

The Secret Gate verifies the live vendor-signed Docker process, ancestry,
arguments, runtime posture, and requested registry. Docker's helper protocol
returns a usable token to the authorized Docker process. A compromised Target
can leak that token.

## Security Boundaries

Automic Vault controls Secret Application and supported sensitive Tool
operations at the Local Execution Boundary. macOS remains responsible for
general process and filesystem security. Root or kernel compromise, arbitrary
local destruction such as `rm -rf`, and a Target's behavior after Secret
Application remain outside the product boundary.

Project Directories select Values, and agent task identifiers narrow Temporary
Access Grants. Neither establishes identity. Authorization History records
local operations but provides neither tamper resistance nor a complete audit
log.

Use macOS TCC as defense in depth. Give a general-purpose terminal or agent
harness Full Disk Access or permission to modify other apps only when its work
requires those permissions.

## Documentation

- [User manual](https://www.automicvault.com/docs/)
- [Domain language](docs/domain-language.md)
- [Architecture](docs/architecture.md)
- [Positioning](docs/positioning.md)
- [Architecture decisions](docs/adr/)
- [Documentation index](docs/index.md)
- [Homebrew tap](https://github.com/automic-vault/homebrew-isotopes)
- [Ephemeral chat](https://outclaw.dev/automic-vault/automic-vault)

Automic Vault is free and open source under Apache-2.0.
