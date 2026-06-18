# Phase 1 Kernel Carve-Out Plan

## Summary

Implement the Phase 1 / first-cuts portion of `REFACTOR.md`: make
`src/lib/rs/lib.rs` mostly module wiring by extracting catalog/data loading and
package runtime state into owned internal modules. Do not split repositories in
this pass.

This is still viable, but keep it deliberately narrow. `lib.rs` has grown well
beyond the original catalog/package-state knot; Phase 1 should not also move
secret scanning, self-update, relocation, isotope migration/post-install logic,
or install execution.

## Key Changes

- Add `src/lib/rs/DOMAIN_MAP.md` mapping current `lib.rs` constants, caches,
  types, and helpers to owners: catalog, package runtime/state, secrets/dotenv,
  trace, runtime boundary, and shared infrastructure.
- Replace broad `use super::*` imports around touched modules with explicit
  imports as needed. Do not churn untouched modules just for style.
- Add a `catalog` module tree for embedded/remote combined data, `Db`, package
  metadata structs, catalog caches, schema validation, remote refresh,
  formula/cask alias indexes, security recommendations, and stub exclusions.
- Add a `package` module tree for package DTOs and persistence: receipt/source
  types, install/search/status/info structs, package selection/request types,
  install plan/options/intent, package mutation lock, root receipt paths,
  ownership manifests, and receipt read/write helpers.
- Keep `config.rs` as the owner of install roots and endpoint roots; remove
  only redundant root wrappers from `lib.rs` after imports are updated.
- Move private tests with the code they verify where practical. Use
  `pub(crate)` only for functions/types consumed across modules, not as a
  blanket escape hatch.

## Interfaces And Compatibility

- No public protocol, helper, XPC, CLI output, receipt, manifest, or `/db.json`
  shape changes.
- Do not bump `DB_SCHEMA_VERSION`, `../av.db/scripts/build-db.py` `SCHEMA_VERSION`,
  `PROTOCOL_VERSION`, `NUKE_PROTOCOL_VERSION`, or `NUKE_HELPER_VERSION`.
- Keep existing crate public exports stable: `main_entry`,
  `scanner_main_entry`, dotenv policy/mode exports, helper command exports,
  isotope entry, and vault entry/types.
- Keep `/db.json` additive/backward-compatible; this refactor only moves Rust
  ownership.

## Implementation Order

1. Commit 1: domain map plus empty module shells and root module wiring.
2. Commit 2: explicit-import prep for modules touched by the extraction.
3. Commit 3: catalog data structs, caches, schema validation, and remote
   refresh, preserving all serde field names and remote-cache behavior.
4. Commit 4: formula/cask alias and catalog lookup helpers.
5. Commit 5: package DTO extraction only, preserving protocol and JSON shapes.
6. Commit 6: package receipt/state helpers, preserving receipt/manifest JSON.
7. Commit 7: cleanup imports/re-exports so `lib.rs` is facade-level wiring plus
   any intentionally retained glue.

Do not move these in Phase 1: secret scanning, self-update, relocation,
generated isotope migration/post-install flows, or install execution.

Leave untracked `REFACTOR.md` as an input artifact unless explicitly asked to
stage it.

## Test Plan

- Refresh the pre/post coverage baseline before starting; the old 93.08% line /
  91.33% region baseline is stale.
- Use a test-only access group when local signing env is not decrypted:
  `AV_DOTENV_KEYCHAIN_ACCESS_GROUP=TESTTEAM.com.automicvault.dotenv`.
- Run `cargo llvm-cov --workspace --summary-only -- --test-threads=1`.
- Run `/usr/bin/python3 scripts/generate-coverage-fixtures.py` and
  `git diff --exit-code -- data/combined.json
  src/lib/rs/fixtures/coverage-combined.json`.
- Run `cargo fmt --check`.
- Run `cargo test --workspace -- --test-threads=1`.
- Run targeted tests after each extraction: catalog/schema/remote-data tests,
  package receipt/state tests, `ops` search/list/info tests, and `protocol`
  package dispatch tests.

## Assumptions

- Scope is Phase 1 only because `REFACTOR.md` is a multi-phase roadmap.
- No new repositories are created in this pass.
- Existing ahead-of-origin commits are user/other-thread work and must not be
  rewritten.
- `PROTOCOL_VERSION` currently lives in `src/lib/rs/core.rs`; this refactor
  should not move or bump it unless behavior changes.
