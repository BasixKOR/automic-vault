# Isotopes

Isotopes are forks of open source projects with explicit approval gates added.
Since these forks must be comprehensive and maintained it is not feasible for
humans to maintain them.

Use agents.

Direct them to this document. Have them make a comprehensive plan for approval
gates. The implementation *must* be minimal because humans still must review
the initial patches and gate positions.

Have the agent leave an `AGENTS.md` in the fork detailing the patches because
we must rebase the patches every new version and we will have agents do this
to ensure the patches remain relevant, secure, and functional.

Isotopes must be forks on the Automic Vault organization. To achieve this,
open a ticket on
[the main repo](https://github.com/automic-vault/automic-vault) and we’ll
create the fork and give you access.

## Approval Gate Criteria

Approval gates should be applied at points where a command crosses a
meaningful risk boundary. The goal is to intercept *evaluated actions*, not
just commands.

### Gate when an action:

#### 1. Is destructive

- Deletes, overwrites, or truncates data
- Examples: `rm`, `delete`, `prune`, `reset --hard`

#### 2. Changes authority or permissions

- Modifies roles, ACLs, or access policies
- Issues credentials or tokens

If the action requires the human to confirm in a browser then this should not
be gated since the browser is already a suitable gate.

#### 3. Exposes secret material

- Prints secrets to stdout
- Writes secrets to disk or environment variables
- Passes secrets to subprocesses

Gate on **egress**, not internal access.

#### 4. Performs external, non-idempotent side effects

- Network writes: `POST`, `PUT`, `DELETE`
- Publishing artifacts, sending messages, triggering webhooks

#### 5. Has a wide blast radius

- Recursive operations, wildcards, or bulk APIs
- Affects more than _N_ resources, using a tool-defined threshold

#### 6. Mutates protected system locations

- Writes outside user-controlled directories, such as `/opt` or
  `/usr/local/bin`
- Installs or modifies executables or services

#### 7. Commits significant cost or compute

- Provisions paid resources
- Triggers long-running or large-scale jobs

## Automic Vault Scope

Automic Vault assumes the agent does not have control over the computer’s
inputs, such as mouse and keyboard. Otherwise the agent could approve itself.
