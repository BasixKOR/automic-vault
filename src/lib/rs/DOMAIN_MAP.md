# `lib.rs` Domain Map

`src/lib/rs/lib.rs` is being reduced to module wiring. Keep moves boring:
preserve names, serde fields, file formats, and protocol payloads.

## Catalog

Owns embedded and remote package data:

- `DB_SCHEMA_VERSION`
- `EMBEDDED_COMBINED_DATA`
- `REMOTE_COMBINED_DATA_*`
- `CombinedData`, `CombinedDataSources`, `RemoteCombinedDataMetadata`
- `Db`
- `EmbeddedFormulaMetadata`, `EmbeddedCaskMetadata`, `EmbeddedCaskBinary`
- `EmbeddedPackagePopularity`, `EmbeddedNpmMetadata`, `EmbeddedNpmPopularity`
- `PackageInstallData`
- `SecurityRecommendationsData`, `SecurityRecommendationPackage`
- `IsotopePackageData`
- `FormulaIndexEntry`
- `embedded_combined_data`
- `refresh_remote_combined_data`
- remote-data trust and write helpers
- formula/cask alias indexes and catalog lookup helpers
- security recommendation and stub exclusion caches

## Package

Owns package runtime DTOs and package state persistence:

- `BREW_PACKAGE_PREFIX`, `CASK_PACKAGE_PREFIX`, `ISOTOPE_PACKAGE_PREFIX`,
  `VENDOR_PACKAGE_PREFIX`
- `ISOTOPE_INSTALL_ROOT_DIR`
- `PKG_STATE_LOCK`
- `ROOT_RECEIPT`, `RECEIPTS_DIR`, `ROOT_EXECUTABLES_MANIFEST`,
  `ROOT_OWNERSHIP_MANIFEST`, `STUB_MANIFEST`, `STUB_HEADER`
- `EmbeddedPackage`, `PackageAliasTarget`, `RequestedPackage`
- `IRequest`, `UninstallRequest`, `UpdateRequest`, `PackageStatusRequest`,
  `InfoRequest`, `SearchRequest`
- `OutputMode`, `PackageSelection`
- `InstallPlan`, `InstallOptions`, `InstallIntent`, `PackageMutationLock`
- `InstallReceipt`, `PackageReceipt`, `PackageReceiptSource`,
  `PackageMetadata`
- `InstalledPackageRecord`, `PackageStatus`, `PackageInfo`,
  `FormulaVersionOption`, `HomebrewPackageInfo`, `PackageSearchResult`,
  `InstalledPackageRef`, `StubManifest`
- package receipt and manifest read/write helpers

## Install Execution

Stays in `lib.rs` for this phase:

- bottle download/staging/install execution
- Homebrew relocation and binary rewriting
- npm/PyPI install commands
- cask/vendor/isotope root installation
- generated isotope migration/post-install flows
- self-update

## Secret Scanner

Stays in `lib.rs` for this phase:

- `SecretScanner*`
- shell-secret probes
- file scanning heuristics and stream output

## Runtime Boundary

Already lives mostly outside `lib.rs`:

- `vault.rs`
- `gate.rs`
- `protocol.rs`
- `core.rs`

Do not move protocol/version behavior during Phase 1.

## Shared Infrastructure

Stays until a later focused pass:

- process/file helpers used by install execution
- progress rendering helpers
- `GLOBAL_TEST_ENV_LOCK`
- `Config`
- `Invocation`
- `Mode`
