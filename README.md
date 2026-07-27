# Automic Vault

> The missing command‐line security‐layer for Mac.

## Quickstart

- Direct download: https://github.com/automic-vault/automic-vault/releases/latest
- Homebrew:
  ```sh
  brew install --cask automic-vault/isotopes/automic-vault \
    && open /Applications/Automic\ Vault.app
  ```
- cURL one-liner:
  ```sh
  curl -fsSL https://www.automicvault.com/install.sh | bash && av open
  # ^^ read it first
  ```

> [!IMPORTANT]
>
> At this time Automic Vault *requires* Homebrew.
> We will loosen this restriction in future, we’re still a pretty new project.

&nbsp;


## What is Automic Vault?

Automic Vault runs in your menu bar detecting existing and emerging
vulnerabilities in your command line tool stacks.

We support (optional) “hardening” steps that typically:

- Moves plaintext secrets into the macOS keychain †
- Installs a `/usr/local/bin` stub as root that federates access to these secrets

> † Thus becoming encrypted at rest, only available to the tools we bless at
> runtime and with approval gates *under our control*.

Hardened tools gain granular controls for execution. You can configure them
to require human approval for specific code-signed application identities or
for all apps/clis at four levels:

1. Approval required for all actions (aka “paranoid mode”)
2. Approval required for actions with side effects (aka “read-only” mode)
3. Approval required for actions that reveal secrets (ie. everything runs except `gh auth token` which blocks at a prompt)
4. No approval required (aka “yolo mode”)

&nbsp;


## How Does Automic Vault Work?

Automic Vault moves secrets out of plaintext files and into the macOS Data
Protection Keychain. Hardened commands request only the named secrets they need;
the menu bar app releases them only when your gate policy allows it or you approve.

For app-specific policy, we walk the process’s launcher chain, validate its code
signature with macOS and match its designated requirement against the identity
you approved. If we cannot verify that identity, automatic approval fails closed.
Code signing proves identity and integrity—not good intentions. You still choose
which apps to trust.

### Blessed scripts

A script can declare the tool access it needs next to its `av inject` shebang:

```sh
#!/usr/local/bin/av inject +TOKEN /bin/sh
# --- automic-vault
# capabilities:
#   gh: read-only
#   aws: trusted
# ---
```

Run `av bless PATH` to review it in the Automic Vault app. Approval is bound to
that canonical path, exact file contents, injection declaration, and the selected
signed launcher apps. While that exact script runs, declared tool requests are
approved up to their listed level; undeclared or broader requests are denied.
Editing the script requires an explicit re-bless.

Blessed scripts run from a verified `/dev/fd/N` snapshot
(to avoid races between approval and potential file edits),
so `$0` is not the
original file path. Automic Vault sets `AV_SCRIPT_PATH` to the canonical path;
use `${AV_SCRIPT_PATH:-$0}` anywhere the script would normally use `$0` to find
files relative to itself. This is done to avoid races between our approval and
potential malicious edits to the script file.

&nbsp;


> [!NOTE]
>
> # Why Automic Vault?
>
> Famously I elected to have Homebrew *not* require `sudo` for `brew install`. A
> controversial decision that was ultimately seen as acknowledging that
> developer environments and *system tools* are different and need different
> security models.
>
> That was then. When the only intelligence using your computer was you, the
> developer, it was reasonable to assume that you were the only one who could
> exfiltrate secrets from your local environment. This assumption is no
> longer valid.
>
> ## The Threat Model Has Changed
>
> - Supply chain attacks target plaintext secrets and other trivial exfiltration
>   mechanisms (eg. calling `gh auth token`)
> - Agents change the game. *No longer are we the only intelligence using our
>   computers.*
>
> Also crucially, Apple have made numerous improvements to the underlying security
> of macOS, eg.:
>
> - System Integrity Protection (SIP)
> - Hardened Runtime
> - Notarization
> - Gatekeeper
> - Privacy protections
> - App Sandbox
>
> **These security measures do not typically apply to command line tools.**
>
> They apply to the `.app` that *runs* the command line tool.
> Which for a developer typically ends up being your terminal.
> Most developers quickly bypass these protections because they
> are too inconvenient for a general purpose tool like a terminal.
>
> Automic Vault is the adapter that applies the macOS operating system’s
> security model to command line tools, while minimizing friction for the
> developer.

&nbsp;


## How Much Friction is This?

As little as possible!

But we aren’t going to lie: it’s more friction than now. We minimize:

- How invasive Automic Vault is.
  - Mostly we install wrappers that change as little as possible.
  - When security requires more, we say so: AWS hardening insists on
    `aws-vault`, which converts your too-powerful AWS keys into short-lived
    session tokens for each invocation.
- We make approval gates rare and smart.
  - By default, secret gates automatically approve read-only commands. Homebrew
    also automatically approves updates (*not* upgrades).
  - Mutating commands still require approval, keeping side effects explicit.
  - Once you get used to that we recommend playing with the levels, eg.
    disabling access to *everything* but one Terminal and your agent apps.
  - We also try to be smart, eg. `gh` even decodes GraphQL queries to determine
    what safety rating they apply to.

&nbsp;


## Important Notes When Using Automic Vault With Agents

If you use agents via their `.app` then you’re good to go: Automic Vault ties
approval gates to codesigned bundles.

If you use agents via their CLI, then the simplest solution is to install the
`.app` version and symlink the CLI that they all bundle to `/usr/local/bin`.
This way Automic Vault can verify the caller via its bundled, notarized code
signature. Automic Vault *will* then trace the executor chain back to the `.app`
and apply the approval gates you set for thatm `.app`.

It is then vital to ensure the harness for the agents has minimal TCC
permissions. If you are using them via CLI that will often be your Terminal

> [!IMPORTANT]
> It is time to go into System Settings and turn off everything you let your
> Terminal do before because it was too tedious not to!
>
> Our suggestion is one terminal for things that may have supply chain attacks
> and one for everything else. The former should be locked down! And yes: this
> means an entirely different app, not just a different version installed to
> a different location. **TCC gates apply to bundle IDs!**
>
> Especially disable:
> - Full Disk Access
> - Modify Other Applications

macOS has come a long way since these OS level permission gates were tedious:
they are quite granular nowadays. And coupled with Automic Vault, they are a
powerful tool to protect your secrets, prevent malware having attack
opportunities and prevent agents from being too dangerous.

> [!NOTE]
> We believe a route to allowing non-codesigned cli tools to have gates is
> doable. We will experiment with that in the near future. However, even if
> possible the code-signed route is more secure and recommended.

### Computer Use & Automic Vault

Computer Use could be used by agents (or malware!) to approve Automic Vault gates.

We provide an iPhone app to mitigate this potential attack vector. The iPhone
app is a companion to Automic Vault that allows you to move Automic Vault gates
from your Macs to your Phone. This way, even if an agent or malware wanted to
approve a request itself, they cannot.

&nbsp;


## User Manual

https://www.automicvault.com/docs/
