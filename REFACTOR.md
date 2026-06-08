# Refactor Plan

## Purpose

This document turns the current architectural read into a concrete refactor
plan.

The goal is not to "clean up the repo" in the abstract. The goal is to make
Automic Vault easier to evolve without weakening its core product promise:

> a local runtime boundary beneath AI agent sessions

That boundary is the product. Everything else exists to support, explain,
package, or distribute it.

## The Core To Preserve

Automic Vault is coherent at the product level. The core system is already
visible in the codebase:

- containment and tool proxying
- secret storage and scoped injection
- human approval gates
- managed package roots and package state
- a local protocol between CLI, app, helper, and daemon surfaces

Those are not optional implementation details. They are the irreducible core.

Any refactor that obscures those boundaries, merges runtimes casually, or
introduces configuration surface where hard-coded trust assumptions are part of
the security model would be a regression.

## The Main Problem

The main issue is not that the product is conceptually confused.

The main issue is that the repository boundary is wider than the product
boundary.

Today, one repository contains at least five materially different machines:

1. The runtime/security core in `src/lib/rs`, `src/nucleus`, `src/helper`, and
   parts of `src/gui`.
2. The native macOS operational surface in `src/gui`.
3. The package-origin web service in `src/web`.
4. The public marketing/docs/SEO site in `www`.
5. The content, research, enrichment, and artifact pipelines in `scripts`,
   `data`, `marketing`, and `research`.

That breadth is manageable while the team is small and the product is still
forming. It becomes expensive when every new change must mentally thread
through all of those concerns at once.

## The Strongest Compression Signal

The clearest structural smell is `src/lib/rs/lib.rs`.

It currently acts as:

- shared prelude
- constant registry
- data-loader surface
- cache holder
- install/runtime substrate
- cross-cutting utility layer
- indirect owner of multiple product domains

That file is large because real work exists, but also because too many concerns
have not yet been forced to declare their actual ownership.

This is the first place to compress.

## Refactor Principles

1. Preserve runtime boundaries.
   CLI, helper, daemon/protocol, GUI, and web origin should remain explicit
   runtimes with clear contracts.

2. Split by responsibility, not by pattern.
   Do not create `utils`, `common`, or arbitrary `*_manager` folders just to
   make files smaller.

3. Keep entrypoints thin.
   Entrypoints should parse, dispatch, and report. They should not silently own
   business logic.

4. Treat persisted formats as architecture.
   Package receipts, manifests, protocol payloads, approval snapshots, and
   `db.json`-derived shapes are contracts, not internal trivia.

5. Prefer additive movement.
   Extract modules behind stable interfaces first. Rename and relocate only
   after seams are proven.

6. Do not widen trust surfaces during cleanup.
   Security-sensitive paths, helper behavior, and production roots are not
   "annoying hard-coding"; they are part of the model.

## Target Shape

The desired end state is not a perfect taxonomy. It is a codebase whose main
domains are obvious.

### 1. Runtime Boundary Domain

Owns:

- containment session setup
- proxy intent capture
- sandbox profile generation
- execution approval request/response flow
- daemon event protocol

Current center of gravity:

- `src/lib/rs/vault.rs`
- `src/lib/rs/gate.rs`

Desired direction:

- a dedicated boundary-oriented module tree such as `src/lib/rs/runtime/` or
  `src/lib/rs/boundary/`
- shared protocol types grouped by runtime concern, not by transport accident

### 2. Package Runtime Domain

Owns:

- package install/update/uninstall
- managed roots
- receipts and root ownership manifests
- package selection and resolution
- package metadata lookup used by CLI and GUI

Current center of gravity:

- `src/lib/rs/ops.rs`
- `src/lib/rs/install.rs`
- `src/lib/rs/state.rs`
- large portions of `src/lib/rs/lib.rs`

Desired direction:

- separate package state, package mutation, and catalog/query code
- `ops.rs` becomes orchestration, not storage of every package concern

### 3. Secret and Dotenv Domain

Owns:

- keychain-backed secret storage
- secret injection rules
- dotenv encryption/decryption flows
- dotenv approval persistence and policy

Current center of gravity:

- `src/lib/rs/dotenv.rs`
- secret-related flows in `vault.rs` and `lib.rs`

Desired direction:

- one explicit surface for secret lifecycle
- one explicit surface for dotenv lifecycle
- shared approval logic reused rather than reimplemented per feature

### 4. Package Catalog and Enrichment Domain

Owns:

- embedded/remote package data loading
- search and category metadata
- isotope/package enrichment
- recommendation and geiger/security catalog data

Current center of gravity:

- large portions of `src/lib/rs/lib.rs`
- `src/lib/rs/info.rs`
- `src/lib/rs/ops.rs`
- scripts in `scripts/`

Desired direction:

- a clearly named catalog/data layer
- data ingestion and runtime query separated from install behavior

### 5. Presentation Surfaces

Owns:

- native app state/view models
- `av-web` request/response rendering
- static web/docs/SEO content

Desired direction:

- native app consumes stable package/runtime interfaces
- `av-web` remains separate from the desktop runtime
- `www`, `marketing`, and `research` are treated as adjacent publishing
  systems, not as part of the product kernel

## What Should Not Be Refactored Together

These concerns should not be mixed into a single sweeping rewrite:

- runtime boundary cleanup
- GUI information architecture cleanup
- website/SEO/content reorganization
- package catalog generation pipeline cleanup
- helper/XPC protocol changes

They touch different risks, different tests, and different release surfaces.
Combining them creates high-noise diffs with low confidence.

## Recommended Sequence

### Phase 1: Carve Out The Kernel

Goal:
make the real product kernel obvious without changing behavior

Work:

- audit `src/lib/rs/lib.rs` and classify its contents by domain ownership
- move pure type definitions and constants into domain-local modules
- extract package catalog loading from package mutation logic
- extract runtime-boundary structs and helpers from generic shared space

Success criteria:

- `lib.rs` becomes mostly module wiring and shared exports
- ownership of major constants and caches becomes obvious
- no protocol or persisted-data behavior changes

### Phase 2: Shrink `ops.rs` Into Orchestration

Goal:
make `ops.rs` coordinate work instead of owning all package behavior

Work:

- split package query/read behavior from mutation behavior
- isolate helper-command execution concerns
- isolate recommendation/search/listing flows from install/update flows
- move formatters or response-shaping helpers closer to their consumers

Success criteria:

- package reads, writes, and catalog queries live in separate modules
- `ops.rs` reads like a use-case layer

### Phase 3: Normalize Approval Flows

Goal:
remove duplicated approval concepts across gate, vault, and dotenv features

Work:

- identify common approval request/decision primitives
- unify persistence and snapshot conventions where possible
- keep runtime-specific fields local, but share approval lifecycle machinery

Success criteria:

- fewer parallel approval implementations
- clearer auditability of what asks for approval and why

### Phase 4: Separate Product Kernel From Publishing Machinery

Goal:
make repo breadth less cognitively expensive

Work:

- define `src/` as the product runtime workspace explicitly
- define `www`, `marketing`, `research`, and content-generation scripts as
  adjacent systems
- optionally move publishing and research workflows into a sibling workspace or
  top-level `site/` or `ops/` grouping if that reduces noise without harming
  deployment

Success criteria:

- contributors can work on runtime code without loading the whole media/SEO
  machine into their head
- release surfaces are easier to reason about

## Concrete First Cuts

If starting immediately, do these first:

1. Write a domain map for `src/lib/rs/lib.rs`.
   Every top-level constant, cache, helper, and type should be assigned an
   owner: boundary, package runtime, catalog, secrets, dotenv, trace, or
   shared infrastructure.

2. Extract catalog/data loading into its own module tree.
   This is high leverage because catalog concerns currently leak into install,
   info, search, and UI-facing code.

3. Extract package state persistence from command orchestration.
   Receipts, manifests, ownership records, and install roots should not be
   discovered indirectly through giant orchestration files.

4. Define one approval vocabulary.
   Not one implementation necessarily, but one naming and lifecycle model.

5. Keep the GUI consuming contracts, not internals.
   Do not let app convenience pull domain logic back across the boundary.

## Anti-Goals

This plan does not aim to:

- introduce a new framework
- split the repository for aesthetic reasons alone
- convert AppKit code to SwiftUI
- make security-sensitive paths configurable
- change public data or protocol contracts casually
- rewrite working code just because it is large

## Risks

1. Accidental protocol drift.
   The GUI and daemon contract is easy to break during "internal" cleanup.

2. Persistence drift.
   Package receipts and manifest files are architecture, not just local state.

3. Security dilution.
   Cleanup pressure can make hard boundaries feel negotiable. They are not.

4. False modularity.
   Moving code into more files without clearer ownership only creates longer
   import graphs and weaker understanding.

## How To Judge A Proposed Change

Before merging a structural change, ask:

1. Does this make the local runtime boundary more explicit or less explicit?
2. Does this reduce the amount of product knowledge hidden inside `lib.rs` or
   `ops.rs`?
3. Does this keep runtime, persistence, and approval contracts legible?
4. Did we move complexity to its owner, or just move it somewhere else?
5. Would a new contributor better understand what the product actually is after
   this change?

If the answer to most of those is "no", it is probably motion, not refactoring.

## Short Version

Automic Vault does not need a reinvention.

It needs compression.

The product kernel is already there. The next job is to make the codebase admit
that truth by separating:

- the boundary from the brochure
- the runtime from the catalog
- orchestration from storage
- approvals from ad hoc feature-specific machinery

That is the path to a codebase that can keep getting stronger without becoming
harder to trust.
