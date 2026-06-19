## Components

1. `av` (internal name: nucleus)
   - A multi-source package manager for Homebrew, npm, PyPI, etc.
   - Installs packages as root to `/opt` and stubs in `/usr/local/bin`.
   - Fast, secure, and optimized for an agentic world.
   - Written in Rust.
2. Automic Vault.app
   - A macOS application for managing Nucleus.
   - Written in Swift/Objective-C with Cocoa/AppKit, not SwiftUI.

Commit as Codex after each completed job.

## Runtime Boundaries

- `src/nucleus/` is the Nucleus CLI runtime.
- `src/lib/rs/` is shared Rust code used by multiple source modules.
- `src/lib/rs/DOMAIN_MAP.md` records the current Rust domain split. Read it
  before moving shared Rust code; update it when ownership changes.
- `src/gui/` is the macOS Cocoa/AppKit application.
- `src/helper/` is privileged/helper code and its launch/XPC support.

`src/lib/rs/lib.rs` is module wiring plus the legacy root test module. Keep
moves boring: preserve names, serde fields, file formats, and protocol payloads.

## `.env`

`.env` is safe to read: sensitive values are encrypted.

If you need to use the decrypted values execute `av dotenv run <SCRIPT>` and
your human will be prompted to approve injecting the project secrets into your
script.

### Versioning

- `NUKE_BUILD_ID` is an automatic exact-build stamp. Do not manually bump it.
  Release and publish builds should derive it from the Git commit; local debug
  builds may use a stable local value to avoid unnecessary Rust rebuilds.
- `NUKE_PROTOCOL_VERSION` tracks the `av serve` protocol contract. Bump it when
  the GUI/helper protocol surface changes, including method names, request
  params, response payloads, required fields, error semantics depended on by the
  GUI, socket lifecycle, or daemon compatibility expectations.
- `NUKE_HELPER_VERSION` tracks the installed privileged helper. Bump it whenever
  privileged helper behavior changes, even if the XPC/protocol interface did not.

## Localization

The app is localized, user-visible strings should be added to
`src/gui/Resources/`.

## Checks

For Rust workspace tests, use
`AV_DOTENV_KEYCHAIN_ACCESS_GROUP=TESTTEAM.com.automicvault.dotenv` unless a task
needs the real keychain access group.
