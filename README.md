# Automic Vault

Security focused base tooling suite for macOS.

[![Download .DMG](https://custom-icon-badges.demolab.com/badge/-Download-blue?style=for-the-badge&logo=download&logoColor=white "Download .DMG")](https://github.com/automic-vault/automic-vault/releases/latest)

> [!NOTE]
>
> - 20k⭐︎: We’ll add Linux support
> - 50k⭐︎: We’ll add Windows support

&nbsp;

> [!IMPORTANT]
>
> Automic Vault is not affiliated with any cryptocurrency or token.

&nbsp;


## Isotopes

Isotopes are forks of open source projects with explicit approval gates added.
Since these forks must be comprehensive and maintained it is not feasible for
humans to maintain them.

Use agents.

Direct them to this README. Have them make a comprehensive plan for approval
gates. The implementation *must* be minimal because humans still must review
the (initial) patches and gate positions.

Have the agent leave an AGENTS.md in the fork detailing the patches because
we must rebase the patches every new version and we will have agents do this
to ensure the patches remain relevant, secure and functional.

Isotopes must be forks on the automic vault organization. To achieve this
open a ticket on
[the main repo](https://github.com/automic-vault/automic-vault) and we’ll
create the fork and give you access.

## Radioisotopes

Radioisotopes exist since some tools cannot be compiled to binaries and thus
we cannot codesign them. They function via `av inject` and thus are less
seamless for the end-user.

Our end goal is to compile radioisotopes to binaries and thus make them
isotopes.

Radioisotopes should be seen thus as temporary.

To add a radioisotope, see the
[radioisotope repo](https://github.com/automic-vault/radioisotopes).

## Next Topes

Keep this list current as new isotopes and radioisotopes land. It should only
include targets that are not already secured.

This list intentionally excludes agent CLIs that are secured by the tools they
run, and excludes dedicated secrets managers that should provide their own
security boundary.

1. `ssh`, `ssh-agent`, and `SSH_AUTH_SOCK`
   - Gate ambient use of unlocked SSH keys by agents and other processes in
     the user's session.
   - Do not replace or delete Apple's `/usr/bin/ssh`; prefer an Automic Vault
     broker, wrappers, and unmanaged-state detection.
2. `brew:kubernetes-cli`
   - Gate production cluster mutation, secret reads, deletes, rollouts, and
     remote execution.
3. `brew:docker`
   - Gate registry credential use, image pushes, privileged containers, and
     host mounts.
4. `git` credential helper and Homebrew `git` integration
   - Gate source-control writes, force pushes, tag mutation, and credential
     use.
   - Do not replace or delete Apple's CLT `git`; control the authentication
     boundary where possible.
5. `brew:azure-cli`
   - Gate cloud resource mutation and Azure identity/token use.
6. `brew:uv`
   - Gate Python package publishing and PyPI token handling.
7. `brew:helm`
   - Gate Kubernetes chart installs, upgrades, deletes, and release mutation.
8. `brew:glab`
   - Gate GitLab source, CI, release, token, and organization administration.
9. `brew:opentofu`
   - Gate infrastructure apply and destroy operations.
10. `brew:ansible`
    - Gate remote fleet mutation and credential use.

&nbsp;


## Criteria for Approval Gates

Approval gates should be applied at points where a command crosses a meaningful
risk boundary. The goal is to intercept *evaluated actions*, not just commands.

### Gate when an action:

#### 1. Is destructive
- Deletes, overwrites, or truncates data
- Examples: `rm`, `delete`, `prune`, `reset --hard`

#### 2. Changes authority or permissions
- Modifies roles, ACLs, or access policies
- Issues credentials or tokens

> If the action requires the human to confirm in a browser then this should
> not be gated since the browser is already a suitable gate.

#### 3. Exposes secret material
- Prints secrets to stdout
- Writes secrets to disk or environment variables
- Passes secrets to subprocesses

> Gate on **egress**, not internal access.

#### 4. Performs external, non-idempotent side effects
- Network writes (POST, PUT, DELETE)
- Publishing artifacts, sending messages, triggering webhooks

#### 5. Has a wide blast radius
- Recursive operations, wildcards, or bulk APIs
- Affects more than _N_ resources (tool-defined threshold)

#### 6. Mutates protected system locations
- Writes outside user-controlled directories (eg. `/opt`, `/usr/local/bin`)
- Installs or modifies executables or services

#### 7. Commits significant cost or compute
- Provisions paid resources
- Triggers long-running or large-scale jobs

&nbsp;


## Considerations for Automic Vault Scope

Automic Vault assumes the agent does not have control over the computer’s
inputs, eg. mouse and keyboard. Since that would allow it to approve itself.
