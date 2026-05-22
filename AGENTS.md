# Automic Vault

## Generalities

Prioritize:
- clear system boundaries
- predictable file layout
- simple tool choices
- additive, low-surprise changes

Do not optimize for novelty. Prefer changes that fit the existing shape of
the repo.

Commit (as Codex) after each completed job.

## Architecture First

Before editing code, identify:
- the user-facing surface being changed
- the runtime boundary involved
- the persistence boundary involved
- whether the change is additive, behavioral, or structural

Prefer preserving established boundaries over introducing new abstractions.

## Repository Shape

Top-level directories should stay few and obvious. Use these meanings:

- `src/`
  Primary application code.
- `src/lib/`
  Shared internal code used by multiple app modules.
- `src/$ENTRYPOINT.$EXT`
  Entrypoints for targets.
- `www/`
  When the product shares a repo with some/all of its website use this directory.
- `scripts/`
  Operational scripts, deploy scripts, maintenance helpers, codegen helpers.
- `tests/`
  Integration tests, black-box tests, and end-to-end tests.
- `data/`
  Local development data, fixtures, SQLite files, generated inventories.
  Always add generated files to `.gitignore`.
- `assets/`
  Source images and static design assets.
- `dist/`
  Build output only. Do not hand-edit.
- `target/`
  Rust build output only. Do not hand-edit.
- `vendor/`
  Checked-in third-party integration modules or package definitions when the
  project intentionally vendors them.
- `docs/`
  Design notes, protocols, architecture docs, and operational references.

Avoid creating new top-level directories unless the boundary is real and
persistent.

## File Layout Rules

- Keep entrypoints thin. Put business logic in reusable modules. Typically
  commands and flags SHOULD be handled in the entrypoint file.
- Keep route definitions, persistence code, and rendering/view code separate.
- Prefer one file per broad responsibility over giant mixed-purpose files.
- When a module becomes hard to scan, split by responsibility, not by pattern.
- Shared utilities belong in `src/lib/` only if used by multiple modules.
- Put operational shell scripts in `scripts/`, not scattered across the repo.

## Tool Choices

Use the simplest toolchain already present in the repo.

### JavaScript / TypeScript
- Prefer Node built-ins where practical.
- Prefer `node --test` for simple test suites.
- Prefer Vite for browser bundling if a bundler is needed.
- Prefer `tsx` for local TS execution when the repo already uses it.
- Prefer SQLite for small single-node products unless the project already uses
  another datastore.

### Rust
- Prefer `cargo test`, `cargo fmt`, and `cargo clippy` when configured.
- Keep reusable code in `src/lib/`.

### Python
- Use `uv` if Python tooling is needed.
- Keep scripts in `scripts/` unless they are importable package code.

### Shell
- Scripts should be non-interactive when possible.
- Put deploy, sync, backup, and publish workflows in `scripts/`.
- Describe deployments in a DEPLOYMENT.md rather than maintaining scripts for
  what would typically be one-off tasks.

Do not add heavy new infrastructure without a clear need.

## Dependency Policy

Prefer:
- standard library
- existing repo dependencies
- small, well-scoped libraries

Avoid adding dependencies for:
- trivial parsing
- small helper functions
- thin wrappers over built-in capabilities

When adding a dependency, keep the reason local and obvious. Justify the
new dependency choice in the README.md under `## Dependencies`.

## Runtime Boundaries

Be explicit about what runs where.

Examples:
- Secrets go in `.envrc` and this MUST be in `.gitignore`.
- server code must not depend on build output internals
- jobs and cron-style tasks should live outside request handlers
- public APIs must remain stable unless v0
- begin API versioning at `v0` when starting new APIs

If the repo has multiple runtimes, keep their code physically separate.

## Persistence Boundaries

Treat the datastore shape as architecture.

- Keep schema-touching code easy to find.
- Keep migrations, seed data, and sync scripts discoverable.
- Prefer additive database changes unless explicitly doing a migration.
- Do not casually rename or repurpose persisted fields.
- Preserve backward compatibility for public data formats and APIs when
  practical.

If the project uses SQLite or local files, assume single-node constraints
matter unless documented otherwise.

## HTTP and Route Conventions

- Reserve machine-facing endpoints under `/api/` unless the repo documents a
  different convention.
- Reserve internal plumbing under a clearly non-public prefix such as `/_/`.
- Keep human-facing routes stable and readable.
- Avoid growing ambiguous top-level route namespaces.

If route naming or URL shape is product-critical, document those invariants in
a project-specific section below.

## Scripts and Operations

Operational workflows should be discoverable from the repo root.

Common examples:
- `scripts/deploy-*`
- `scripts/sync-*`
- `scripts/backup-*`
- `scripts/publish-*`
- `scripts/generate-*`

If an operation is important enough to do twice, prefer a script over tribal
knowledge.

## Generated and Checked-In Artifacts

- `dist/`, `target/`, caches, and temporary outputs are generated artifacts.
- Do not edit generated files unless the repo intentionally treats them as
  source.
- If a generated file is committed, update the source that produces it when
  possible, then regenerate.

Document any exceptions in the project-specific notes.

## Testing and Verification

Before finishing substantial work, run the smallest relevant verification:
- targeted tests first
- then broader checks if needed
- then a build if the change affects packaging or deployability

Prefer fast local verification over exhaustive but slow workflows.

## Change Discipline

Prefer:
- additive changes
- narrow diffs
- preserving public contracts
- preserving file layout conventions

Avoid:
- opportunistic rewrites
- introducing new frameworks casually
- moving many files without architectural benefit
- mixing unrelated refactors into feature work

## Quick Start for Agents

When starting work:
1. Read this file and scan the repo root.
2. Identify the runtime and persistence boundaries involved.
3. Find the nearest existing module that already owns this responsibility.
4. Make the smallest change that fits the current structure.
5. Verify with the narrowest relevant command.

## Project Specific Notes

- Update this section with new facts about the project as they arise.
- Do not put general engineering advice here.
- Keep this section up-to-date.

### Components

1. Nucleus
   - A multi-source package manager for Homebrew, npm and pypi packages.
   - Installs as root to /opt and stubs in /usr/local/bin.
   - Fast, secure and optimized for an agentic world.
   - Written in Rust.
   - Keep coverage above 80%
2. Automic Vault.app
   - A macOS application that allows managing Nucleus.
   - Written in Swift/ObjC (Cocoa NOT SwiftUI)

### Security

- Nucleus must not expose runtime-configurable install roots or package
  metadata endpoints. Keep production paths and trusted upstreams hard-coded.
- Release builds install under `/opt` and stub into `/usr/local/bin`.
- Debug builds install under `/tmp/opt` and stub into
  `/tmp/usr/local/bin`.
- Treat new configurability surfaces as security-sensitive because they can
  introduce exploit vectors.

### File Layout

- ./www/ web site, static HTML
- ./src/lib/rs   # shared rust code
- ./src/nucleus
- ./src/gui

### GUI User Experience

- no click should ever feel like nothing happens, if the response cannot be
  instant: show a spinner or loading animation
- the main thread can never block more than 8.5ms

### Component Versioning

- gui and cli tools all have the same version from Cargo.toml
- helpers and protocols have their own versions and must be bumped independently when their interface changes
- Bump `NUKE_HELPER_VERSION` in `.env` whenever privileged helper behavior
  changes, even if the XPC/protocol interface is unchanged. Installed helpers
  are only replaced when the bundled helper version is newer.
- Bump `NUKE_PROTOCOL_VERSION` only when the GUI/helper protocol surface
  changes, such as XPC methods, wire payloads, or protocol daemon contracts.
