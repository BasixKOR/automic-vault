# `lib.rs` Domain Map

`src/lib/rs/lib.rs` is module wiring plus the legacy root test module. Keep
moves boring: preserve names, serde fields, file formats, and protocol payloads.

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

Owns package installation execution:

- bottle download/staging/install execution
- Homebrew relocation and binary rewriting
- npm/PyPI install commands
- cask/vendor/isotope root installation
- generated isotope migration/post-install flows
- self-update
- post-install hook wiring

## Secret Scanner

Owns scanner CLI execution and shell/file probes:

- `SecretScanner*`
- shell-secret probes
- file scanning heuristics and stream output

## Isotope

Owns isotope runtime helpers:

- isotope package lookup and virtual versioned isotope records
- isotope target/modification/replacement resolution
- generated isotope integration dispatch
- isotope security state helpers

## Runtime Boundary

Lives outside `lib.rs`:

- `vault.rs`
- `gate.rs`
- `protocol.rs`
- `core.rs`

## Shared Infrastructure

Lives in focused modules:

- process/file helpers used by install execution: `install.rs`
- progress rendering helpers: `install.rs`
- `GLOBAL_TEST_ENV_LOCK`: `core.rs`
- `Config`, `Invocation`, `Mode`, `ProgressCallback`: `core.rs`
