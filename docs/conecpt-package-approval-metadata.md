# Package Approval Metadata

This document designs a fork-scalable metadata repository for package command
approval policy. The repository is only data. Automic Vault, Nucleus, isotope
forks, and other tools can consume it, but the metadata repo must not become a
runtime plugin system.

The goal is to let contributors describe, per package, which command shapes
cross meaningful risk boundaries and what consequences those commands can have.
Users then choose how anxious Automic Vault should be, and the tool maps the
metadata to allow, warn, require approval, or block decisions.

## Architecture Boundary

User-facing surface:

- Package risk and approval prompts in Automic Vault.
- CLI or GUI explanations for why a command is being gated.
- Contributor PRs that add or refine package command metadata.

Runtime boundary:

- The metadata repo is read-only policy input.
- Automic Vault/Nucleus owns policy interpretation and approval UI.
- Isotopes own executable enforcement points.
- Package managers and CLIs remain untrusted upstream software.

Persistence boundary:

- Package manifests live in a separate Git repository.
- Local tools may cache signed snapshots of that repository.
- User anxiety settings and user decisions live in Automic Vault state, not in
  the metadata repo.

Change type:

- Additive design. It introduces a metadata contract without changing current
  gate behavior.

## Repository Model

Use one public repository, for example:

```text
automic-vault/package-approval-metadata
```

The repo is optimized for forks:

- one package per YAML file
- ecosystem folders at the root
- stable schema versioning
- no generated files required for normal contributions
- validation in CI
- owners can review narrow package diffs independently

Recommended layout:

```text
AGENTS.md
README.md
schema/
  package-approval-manifest.v0.schema.json
  package-approval-index.v0.schema.json
brew/
  awscli.yaml
  docker.yaml
  kubernetes-cli.yaml
cask/
  docker.yaml
npm/
  @anthropic-ai/
    claude-code.yaml
  vercel.yaml
pip/
  ansible.yaml
  twine.yaml
isotope/
  gh.yaml
  aws-cli.yaml
tests/
  fixtures/
  validate-manifests.ts
```

Root folders are package namespaces. They should match the source namespace a
user installs from:

- `brew/` for Homebrew formulae.
- `cask/` for Homebrew casks.
- `npm/` for npm packages.
- `pip/` for Python packages published to PyPI.
- `isotope/` for Automic Vault isotope package names.

Additional namespaces can be added only when the package identity boundary is
real and persistent, such as `cargo/`, `gem/`, `go/`, or `github-release/`.

## Path Rules

Package file paths must be predictable and stable.

```text
<namespace>/<normalized-package-name>.yaml
```

Normalization:

- Preserve lowercase package names where the ecosystem is case-sensitive only
  by convention.
- For npm scopes, use directories: `npm/@scope/name.yaml`.
- For Homebrew taps outside `homebrew/core`, use tap directories only when the
  tap is part of the install identity: `brew/homebrew/cask/docker.yaml` is not
  needed for core packages, but `brew/vendor/tap/name.yaml` may be.
- Do not encode versions in filenames.
- Do not combine multiple ecosystem packages in one manifest.

## Trust Model

Metadata is advisory until a trusted runtime enforces it. A malicious manifest
must not be able to execute code, fetch dynamic rules, or weaken hard-coded
security boundaries.

Consumers must treat manifests as:

- untrusted data
- schema-validated
- bounded in size and complexity
- signed or pinned by snapshot when used for automatic gating

Manifests may describe risky command shapes, but they must not grant authority.
For example, a manifest can say `gh repo delete` is destructive remote mutation.
It cannot say that this command is always safe for a user.

## Concepts

### Package

A package manifest describes one installable package in one namespace.

### Entrypoint

An executable, script, service, daemon, or GUI helper exposed by the package.
Most manifests will define one or more command-line entrypoints.

### Command Shape

A command shape is a structured match over argv, options, environment, stdin,
path targets, and sometimes inferred context.

Examples:

- `git push --force`
- `kubectl delete namespace prod`
- `npm publish`
- `aws s3 rm --recursive s3://bucket`
- `docker run --privileged -v /:/host`

### Consequence

A consequence is what can happen if the command succeeds. Consequences are
separate from command names so different tools can share the same user policy.

Examples:

- remote write
- local write
- local delete
- secret egress
- credential issuance
- privilege escalation
- paid resource creation
- source-control history rewrite

### Gate

A gate is the recommended approval behavior for a command shape at different
user anxiety levels. The final decision belongs to the consuming tool and user
settings.

## Anxiety Levels

The metadata should support a small, stable set of user anxiety levels. These
levels describe how often the user wants to be interrupted.

```yaml
anxietyLevels:
  relaxed:
    intent: "Gate only irreversible or credential-exposing actions."
  normal:
    intent: "Gate remote writes, destructive local writes, and secret egress."
  cautious:
    intent: "Gate broad local writes, identity use, and high-blast-radius reads."
  strict:
    intent: "Gate most mutation and any ambiguous high-authority command."
  locked:
    intent: "Default deny unless a command shape is explicitly allowed."
```

Automic Vault should keep the canonical level names in its own code. Manifests
can reference them but should not define new levels.

## Gate Actions

Use a small vocabulary so tools can produce predictable UI:

- `allow`: run without interruption
- `notify`: run and show a passive event
- `approve_once`: require approval for this execution
- `approve_session`: require approval, then allow similar executions for the
  current agent session when the user chooses
- `approve_persistent`: allow a persistent exception only if the executable
  boundary is root-controlled and the consumer supports persistent approvals
- `block`: deny by default; approval UI may omit an allow button

Manifests recommend actions. Consumers may always choose a stricter action.

## Consequence Taxonomy

The taxonomy should be shared across all namespaces. Keep it stable and add
new values carefully.

### Local Effects

- `local_read`: reads local files or directories
- `local_write`: creates or modifies local files
- `local_delete`: deletes local files or directories
- `local_overwrite`: replaces existing local content
- `local_execute`: runs a local executable, hook, script, or plugin
- `system_write`: writes outside normal user data locations
- `service_mutation`: installs, starts, stops, or modifies background services
- `permission_change`: changes file modes, ownership, ACLs, roles, or policies

### Secret and Identity Effects

- `secret_read`: accesses secret material
- `secret_egress`: prints, writes, transmits, or passes secrets to another
  process
- `credential_issue`: creates, refreshes, exports, or exchanges credentials
- `identity_use`: performs an authenticated action as a user, service account,
  or organization
- `auth_boundary_change`: changes login, SSO, trust, or credential-helper state

### Remote Effects

- `network_read`: reads from a remote service
- `remote_write`: creates, modifies, or deletes remote state
- `remote_delete`: deletes remote resources
- `remote_execute`: executes code or commands on remote infrastructure
- `publish`: publishes packages, images, artifacts, releases, or messages
- `source_history_rewrite`: rewrites source-control history, tags, refs, or
  protected branches
- `webhook_trigger`: triggers automation, deployment, CI, notification, or
  callback behavior

### Blast Radius Effects

- `recursive`: recursively affects many resources
- `wildcard`: uses glob, selector, all, any, or namespace-wide targeting
- `bulk`: affects more than a package-specific threshold
- `production`: targets production or a similarly sensitive environment
- `cross_account`: affects a different account, organization, tenant, or
  project than the local default
- `irreversible`: cannot be cleanly undone by the same tool

### Cost and Supply Chain Effects

- `cost_commit`: provisions paid resources or triggers billable compute
- `long_running_compute`: starts expensive or long-lived jobs
- `supply_chain_mutation`: changes dependency, lockfile, registry, package,
  image, signing, or release state
- `trust_policy_change`: changes signing keys, provenance, attestation, or
  policy settings

## Manifest Schema

Each manifest has these top-level sections:

```yaml
schemaVersion: package-approval-manifest.v0
package:
  namespace: brew
  name: awscli
  displayName: AWS CLI
  upstream:
    homepage: https://aws.amazon.com/cli/
    source: https://github.com/aws/aws-cli
  replaces: []
  aliases:
    - aws

coverage:
  status: partial
  reviewedAt: "2026-05-21"
  reviewedBy:
    - github:example
  notes:
    - "Covers high-risk mutating operations and secret egress. Read-only APIs are not exhaustively modeled."

entrypoints:
  - name: aws
    kind: cli
    argvGrammar: positional-subcommands
    commandTree:
      # See command tree section.

rules:
  # See rule section.
```

Required fields:

- `schemaVersion`
- `package.namespace`
- `package.name`
- `coverage.status`
- `entrypoints`
- `rules`

Optional fields should be additive. Consumers must ignore unknown fields only
when the schema version explicitly allows extensions.

## Coverage Status

Coverage is a contributor honesty mechanism. It helps Automic Vault distinguish
known safe areas from unknown areas.

Allowed values:

- `stub`: package identity exists but command risk is not described
- `partial`: important command shapes are covered
- `broad`: most high-risk command shapes are covered
- `exhaustive`: command grammar is modeled deeply enough for strict policy

Unknown command shapes should become more suspicious as user anxiety rises.

Recommended default handling:

```yaml
unknownCommandPolicy:
  relaxed: allow
  normal: notify
  cautious: approve_once
  strict: approve_once
  locked: block
```

## Command Tree

The command tree makes package behavior reviewable without requiring a full
parser. It is documentation plus optional structured hints for consumers.

```yaml
entrypoints:
  - name: gh
    kind: cli
    argvGrammar: positional-subcommands
    globals:
      options:
        - names: ["--repo", "-R"]
          value: repository
        - names: ["--hostname"]
          value: hostname
    commandTree:
      repo:
        description: Repository operations
        subcommands:
          delete:
            summary: Delete a repository
            consequences: [remote_delete, irreversible, identity_use]
          edit:
            summary: Change repository settings
            consequences: [remote_write, identity_use]
      auth:
        subcommands:
          token:
            summary: Print the active token
            consequences: [secret_egress]
```

The command tree should prefer clarity over completeness. Enforcement should
come from `rules`, because real CLIs often have aliases, global flags, and
option combinations that a tree alone cannot express.

## Rules

Rules are ordered. The first matching rule may decide the consequence and gate
recommendation, or consumers may merge all matching rules and choose the
strictest result. The schema should require each rule to declare its intended
composition behavior.
More specific `terminal` exceptions should appear before broader rules.

```yaml
rules:
  - id: gh.repo.delete
    description: Delete a GitHub repository.
    entrypoint: gh
    match:
      argv:
        startsWith: ["repo", "delete"]
    consequences:
      - type: remote_delete
        resource: github.repository
      - type: irreversible
      - type: identity_use
        identity: github.account
    severity: critical
    confidence: high
    gate:
      relaxed: approve_once
      normal: approve_once
      cautious: approve_once
      strict: approve_once
      locked: block
    explain:
      summary: "Deletes a GitHub repository."
      prompt: "Allow gh to delete a GitHub repository?"
```

### Rule Fields

Required:

- `id`: stable package-local identifier
- `description`: contributor-readable explanation
- `entrypoint`: executable name from `entrypoints`
- `match`: structured matcher
- `consequences`: list of consequences
- `severity`: `info`, `low`, `medium`, `high`, or `critical`
- `confidence`: `low`, `medium`, or `high`
- `gate`: anxiety-level action map
- `explain.summary`: short UI text

Optional:

- `explain.prompt`: approval prompt text
- `examples`: argv examples that should match
- `counterExamples`: argv examples that should not match
- `references`: upstream docs or source links
- `thresholds`: package-specific blast radius thresholds
- `composition`: `terminal`, `merge`, or `continue`

## Match Language

The match language must stay declarative and bounded. It should be expressible
as JSON Schema and evaluated without executing package code.

### Argv Matchers

```yaml
match:
  argv:
    startsWith: ["push"]
    containsAny: ["--force", "--force-with-lease"]
    containsAll: ["--tags"]
    option:
      name: "--output"
      valueMatches: "^-?$"
```

Supported primitives:

- `equals`: exact argv sequence
- `startsWith`: argv prefix
- `contains`: exact token
- `containsAny`: any token in a list
- `containsAll`: all tokens in a list
- `subcommand`: normalized subcommand path
- `option.name`: flag or option name
- `option.valueEquals`: exact option value
- `option.valueMatches`: constrained regular expression
- `positional.index`: positional argument index after parsed subcommands
- `positional.matches`: constrained regular expression

Regular expressions must be length-limited and use a safe regex engine.

### Boolean Matchers

```yaml
match:
  all:
    - argv:
        startsWith: ["run"]
    - argv:
        contains: "--privileged"
  any:
    - argv:
        contains: "-v"
    - argv:
        contains: "--mount"
  not:
    argv:
      contains: "--dry-run"
```

### Path Matchers

Path matchers describe local filesystem consequences.

```yaml
match:
  pathTargets:
    any:
      - fromOption: "--output"
        class: system
      - fromPositional: 0
        class: cwd
```

Path classes are consumer-defined but should include:

- `cwd`: inside current working directory
- `home`: inside the user's home directory
- `secrets`: common credential and key locations
- `system`: protected system locations such as `/opt`, `/usr/local/bin`,
  `/Library`, and `/etc`
- `external-volume`: removable or network-mounted storage
- `unknown`: cannot classify safely

### Environment Matchers

```yaml
match:
  env:
    containsAny:
      - AWS_PROFILE
      - AWS_ACCESS_KEY_ID
    valueMatches:
      AWS_PROFILE: "prod|production"
```

Environment matching is useful for production detection, credential exposure,
and cloud account targeting. It should not require consumers to inspect secret
values.

### Stdin and TTY Matchers

```yaml
match:
  stdin:
    mayContainSecret: true
  tty:
    interactive: false
```

Use this sparingly. Tools often cannot know stdin content safely. Prefer
declaring uncertainty and gating more strictly at higher anxiety levels.

## Severity Guidance

Use severity to communicate blast radius, not vibes.

- `info`: visibility only; no mutation or secret movement
- `low`: local mutation with narrow scope and straightforward recovery
- `medium`: local destructive action, authenticated read, or narrow remote write
- `high`: remote mutation, secret egress, production targeting, or broad local
  mutation
- `critical`: irreversible deletion, credential issuance, policy/admin change,
  source history rewrite, or high-cost infrastructure mutation

## Gate Selection Rules

Manifest authors should start from consequence defaults, then adjust for
package-specific context.

Suggested defaults:

```yaml
consequenceDefaults:
  local_read:
    relaxed: allow
    normal: allow
    cautious: notify
    strict: approve_once
    locked: block
  local_write:
    relaxed: allow
    normal: notify
    cautious: approve_once
    strict: approve_once
    locked: block
  local_delete:
    relaxed: notify
    normal: approve_once
    cautious: approve_once
    strict: approve_once
    locked: block
  system_write:
    relaxed: approve_once
    normal: approve_once
    cautious: approve_once
    strict: approve_once
    locked: block
  secret_egress:
    relaxed: approve_once
    normal: approve_once
    cautious: approve_once
    strict: approve_once
    locked: block
  remote_write:
    relaxed: notify
    normal: approve_once
    cautious: approve_once
    strict: approve_once
    locked: block
  remote_delete:
    relaxed: approve_once
    normal: approve_once
    cautious: approve_once
    strict: approve_once
    locked: block
  publish:
    relaxed: approve_once
    normal: approve_once
    cautious: approve_once
    strict: approve_once
    locked: block
  source_history_rewrite:
    relaxed: approve_once
    normal: approve_once
    cautious: approve_once
    strict: approve_once
    locked: block
  cost_commit:
    relaxed: approve_once
    normal: approve_once
    cautious: approve_once
    strict: approve_once
    locked: block
```

Rules can be stricter than defaults. They should be looser only when upstream
behavior is well documented and examples prove the lower risk.

## Example: npm Publish

```yaml
schemaVersion: package-approval-manifest.v0
package:
  namespace: npm
  name: npm
  displayName: npm CLI
  upstream:
    homepage: https://docs.npmjs.com/cli/
  aliases: [npm, npx]

coverage:
  status: partial
  reviewedAt: "2026-05-21"

entrypoints:
  - name: npm
    kind: cli
    argvGrammar: positional-subcommands
    commandTree:
      publish:
        summary: Publish a package to the npm registry
        consequences: [publish, remote_write, identity_use, supply_chain_mutation]
      unpublish:
        summary: Remove package versions from the npm registry
        consequences: [remote_delete, supply_chain_mutation, irreversible]
      token:
        subcommands:
          create:
            consequences: [credential_issue, identity_use]
          revoke:
            consequences: [auth_boundary_change, remote_write]

rules:
  - id: npm.publish.dry-run
    description: npm publish with --dry-run does not write to the registry.
    entrypoint: npm
    composition: terminal
    match:
      all:
        - argv:
            startsWith: ["publish"]
        - argv:
            contains: "--dry-run"
    consequences:
      - type: local_read
      - type: local_execute
        note: "Lifecycle scripts may run unless disabled by npm behavior/version."
    severity: medium
    confidence: medium
    gate:
      relaxed: allow
      normal: notify
      cautious: approve_once
      strict: approve_once
      locked: block
    explain:
      summary: "Simulates an npm publish and may run package scripts."

  - id: npm.publish
    description: Publish the current package or tarball to the npm registry.
    entrypoint: npm
    match:
      argv:
        startsWith: ["publish"]
    consequences:
      - type: publish
        resource: npm.package
      - type: remote_write
        resource: npm.registry
      - type: identity_use
        identity: npm.account
      - type: supply_chain_mutation
    severity: critical
    confidence: high
    gate:
      relaxed: approve_once
      normal: approve_once
      cautious: approve_once
      strict: approve_once
      locked: block
    explain:
      summary: "Publishes a package to npm."
      prompt: "Allow npm to publish a package to the registry?"
    examples:
      - argv: ["npm", "publish"]
      - argv: ["npm", "publish", "./pkg.tgz", "--access", "public"]
    counterExamples:
      - argv: ["npm", "pack"]
```

## Example: Docker Privileged Host Access

```yaml
schemaVersion: package-approval-manifest.v0
package:
  namespace: brew
  name: docker
  displayName: Docker CLI
  upstream:
    homepage: https://docs.docker.com/reference/cli/docker/
  aliases: [docker]

coverage:
  status: partial
  reviewedAt: "2026-05-21"

entrypoints:
  - name: docker
    kind: cli
    argvGrammar: positional-subcommands

rules:
  - id: docker.run.privileged
    description: Run a container with elevated host privileges.
    entrypoint: docker
    match:
      all:
        - argv:
            startsWith: ["run"]
        - argv:
            contains: "--privileged"
    consequences:
      - type: permission_change
      - type: system_write
      - type: local_execute
      - type: identity_use
        identity: docker.daemon
    severity: critical
    confidence: high
    gate:
      relaxed: approve_once
      normal: approve_once
      cautious: approve_once
      strict: approve_once
      locked: block
    explain:
      summary: "Runs a container with elevated host privileges."

  - id: docker.run.host-root-mount
    description: Mount the host root filesystem into a container.
    entrypoint: docker
    match:
      all:
        - argv:
            startsWith: ["run"]
        - any:
            - argv:
                contains: "-v"
            - argv:
                contains: "--volume"
            - argv:
                contains: "--mount"
        - pathTargets:
            any:
              - class: system
              - literal: "/"
    consequences:
      - type: local_read
      - type: local_write
      - type: secret_read
      - type: secret_egress
      - type: system_write
    severity: critical
    confidence: medium
    gate:
      relaxed: approve_once
      normal: approve_once
      cautious: approve_once
      strict: approve_once
      locked: block
    explain:
      summary: "Mounts sensitive host paths into a container."
```

## Example: kubectl Deletes

```yaml
schemaVersion: package-approval-manifest.v0
package:
  namespace: brew
  name: kubernetes-cli
  displayName: kubectl
  upstream:
    homepage: https://kubernetes.io/docs/reference/kubectl/
  aliases: [kubectl]

coverage:
  status: broad
  reviewedAt: "2026-05-21"

entrypoints:
  - name: kubectl
    kind: cli
    argvGrammar: positional-subcommands
    globals:
      options:
        - names: ["--context"]
          value: kubernetes.context
        - names: ["--namespace", "-n"]
          value: kubernetes.namespace

rules:
  - id: kubectl.delete
    description: Delete Kubernetes resources.
    entrypoint: kubectl
    match:
      argv:
        startsWith: ["delete"]
    consequences:
      - type: remote_delete
        resource: kubernetes.resource
      - type: identity_use
        identity: kubernetes.user
    severity: high
    confidence: high
    gate:
      relaxed: notify
      normal: approve_once
      cautious: approve_once
      strict: approve_once
      locked: block
    explain:
      summary: "Deletes Kubernetes resources."

  - id: kubectl.delete.namespace
    description: Delete a Kubernetes namespace.
    entrypoint: kubectl
    composition: merge
    match:
      argv:
        startsWith: ["delete", "namespace"]
    consequences:
      - type: remote_delete
        resource: kubernetes.namespace
      - type: recursive
      - type: irreversible
    severity: critical
    confidence: high
    gate:
      relaxed: approve_once
      normal: approve_once
      cautious: approve_once
      strict: approve_once
      locked: block
    explain:
      summary: "Deletes a Kubernetes namespace and its contained resources."

  - id: kubectl.production-context
    description: Command targets a production-like Kubernetes context.
    entrypoint: kubectl
    composition: merge
    match:
      any:
        - argv:
            option:
              name: "--context"
              valueMatches: "(^|[-_.])prod(uction)?($|[-_.])"
        - env:
            valueMatches:
              KUBECONFIG: "prod|production"
    consequences:
      - type: production
    severity: high
    confidence: medium
    gate:
      relaxed: notify
      normal: approve_once
      cautious: approve_once
      strict: approve_once
      locked: block
    explain:
      summary: "Targets a production-like Kubernetes context."
```

## Unknowns and Ambiguity

Manifests should explicitly model ambiguity instead of pretending every command
can be known statically.

Useful fields:

```yaml
ambiguity:
  dynamicPlugins: true
  readsConfigFiles:
    - "~/.aws/config"
    - "~/.kube/config"
  remoteApiSurface: large
  notes:
    - "Subcommands can be extended by plugins."
```

Consumer behavior:

- relaxed users tolerate more unknowns
- normal users get notified for unknown remote-capable commands
- cautious and strict users approve unknown mutation-capable commands
- locked users block unknown command shapes unless locally allowlisted

## Contributor Workflow

1. Fork the metadata repo.
2. Add or edit one package YAML file.
3. Include examples and counterexamples for each rule.
4. Run schema validation.
5. Open a PR with links to upstream command docs or source.
6. Reviewers check that consequences, severity, and gate recommendations match
   documented behavior.

PRs should be small:

- one ecosystem package per PR when possible
- broad packages split by subcommand area
- no drive-by schema changes with package coverage changes

## CI Validation

The metadata repo should validate:

- YAML parses cleanly.
- Schema version is known.
- File path matches `package.namespace` and `package.name`.
- Rule IDs are unique within the manifest.
- `entrypoint` references are valid.
- Gate maps include every canonical anxiety level.
- Consequence values are from the shared taxonomy.
- Severity and gate actions are known.
- Examples match their rule.
- Counterexamples do not match their rule.
- Regular expressions compile with the supported safe regex engine.
- Manifest size and rule count stay under published limits.

CI should not require live network access for normal PR validation.

## Snapshot and Distribution

Automic Vault should consume signed snapshots, not arbitrary branch heads.

Recommended flow:

1. Metadata repo receives contributor PRs.
2. CI validates all manifests.
3. Maintainers tag a release.
4. Release automation produces:
   - a normalized JSON index
   - a content hash manifest
   - a signature
5. Automic Vault/Nucleus updates from pinned release metadata.

Local cache path should stay under Automic Vault application support state.
The cache should be replaceable and should not contain user decisions.

## Consumer Algorithm

At command execution time:

1. Identify package namespace and package name.
2. Load the matching manifest from the signed local snapshot.
3. Normalize argv according to the manifest entrypoint grammar.
4. Evaluate rules.
5. Merge consequences using rule composition.
6. Select the strictest recommended gate for the user's anxiety level.
7. Apply local user exceptions only if the executable boundary is trusted.
8. Present a prompt that names:
   - executable
   - command
   - package
   - consequences
   - target resources when known
   - parent process or agent session
9. Record the user's decision outside the metadata repo.

Consumers must fail closed only for high-anxiety modes or hard security
boundaries. For normal modes, missing metadata should not break ordinary tools.

## Relationship to Isotopes

This metadata does not replace isotopes.

Use metadata for:

- discovering high-risk command shapes
- explaining approval prompts
- helping contributors plan isotope gates
- ranking packages by uncovered risk
- building UI that adapts to user anxiety

Use isotopes for:

- placing actual runtime gates inside tools
- intercepting evaluated actions after config and aliases resolve
- protecting secret egress and credential use
- enforcing gates where argv matching is insufficient

When both exist, isotope instrumentation should be treated as stronger evidence
than metadata-only argv matching.

## Security Constraints

The metadata system must not:

- make install roots runtime-configurable
- make trusted upstream package endpoints runtime-configurable
- execute manifest-provided code
- allow manifests to disable hard-coded gates
- allow package maintainers to silently downgrade critical consequences
- persist user approval choices in the shared repo
- require users to trust every fork

Automic Vault may accept community metadata quickly, but enforcement semantics
must remain owned by Automic Vault.

## Initial Package Targets

Start with packages where command metadata is valuable even before full isotope
coverage exists:

- `brew/kubernetes-cli.yaml`
- `brew/docker.yaml`
- `brew/gh.yaml`
- `brew/azure-cli.yaml`
- `brew/helm.yaml`
- `brew/opentofu.yaml`
- `brew/ansible.yaml`
- `npm/npm.yaml`
- `npm/pnpm.yaml`
- `npm/yarn.yaml`
- `pip/twine.yaml`
- `pip/ansible.yaml`

These packages have clear remote write, publish, credential, or infrastructure
mutation surfaces and are likely to be used by autonomous coding agents.

## Open Questions

- Whether `brew` and `cask` should be separate namespaces or separate package
  kinds under a shared Homebrew namespace.
- How much command parsing Automic Vault should do before requiring isotope
  instrumentation.
- Whether package manifests should support localized prompt text.
- How package metadata should reference specific CLI versions when behavior
  changes.
- Whether strict mode should block unknown command shapes for partially covered
  manifests or only for `locked` mode.
