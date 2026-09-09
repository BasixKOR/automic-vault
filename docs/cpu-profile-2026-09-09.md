# CPU investigation, 9 September 2026

The midnight session reduced Authorization History decoding and eliminated
Launcher discovery for no-Secret scripts with an empty capability ceiling.
Signature validation remains the main observed CPU cost during repeated
requests. The measurements below do not establish a reduction in
total CPU for Secret exchange with Isotopes.

This report uses the definitions in [domain language](domain-language.md),
[architecture](architecture.md), and [positioning](positioning.md).

Branch: `codex/overnight-cpu-20260909`, in `/Users/mxcl/src/av-cpu-20260909`.
Implementation commits:

- `75d6e390`: verify persisted history without decoding it twice.
- `3ab7b54d`: skip catalog reads for ineligible helper candidates.
- `ec616c8a`: defer Launcher discovery for empty-capability script starts.

## Changes

- Verify the complete persisted Authorization History bytes after each append,
  instead of decoding all 50 records again and comparing only the newest UUID.
  The app still persists and verifies the Authorization Record before releasing
  Secrets. The lock, storage destination, and Keychain accessibility are unchanged.
- Reject non-Developer-ID and non-app helper candidates before reading the
  Launcher helper catalog. Eligible helpers still undergo the same catalog,
  signature, runtime, and resource-seal checks.
- Defer ordinary Launcher discovery until after the existing no-Secret script
  path for an explicit empty capability ceiling. Gate Client verification and
  Launcher Bundle integrity checks still precede dispatch. Request normalization,
  credential binding, and ceiling registration retain their existing order.
  GPG still discovers Launchers before selecting its credential; SSH still uses
  its verified peer Launchers. Requests that need authorization still discover
  Launchers before evaluating policy.

## Measurements

Release builds on macOS 26.6.2 (25G83), Xcode 26.6 (17F113). The branch starts at
`bc3ccf5c`; paired comparisons isolate each change. Fixtures contain synthetic
metadata and request no real Secrets.
CPU means process user plus system CPU time; paired runs alternate before/after.

| Workload | Before median CPU | After median CPU | Result |
| --- | ---: | ---: | --- |
| History fixture, 50 warm-up + 500 appends, seven pairs | 448 ms | 318 ms | 29% lower |
| Launcher ancestry, 30 scans, five pairs | 9.442 s | 9.424 s | Within measurement noise |
| Empty-ceiling inject handler, 30 calls, five pairs | 9.594 s | 0.020 s | Launcher discovery eliminated |

History fixture wall time fell from 500 ms to 357 ms. These figures include
standalone executable startup and use the existing UserDefaults test seam.
They measure serialization and persistence verification overhead, not production
Data Protection Keychain latency. An isolated signed Keychain fixture could not
run because it lacked the required entitlement (`-34018`).

The empty-ceiling fixture invokes `handleInject` with a synthetic script and
asserts that each call returns success with an empty Secret payload and registers
the live execution's empty capability ceiling. Median wall time fell from
10.240 s to 0.096 s, including executable startup. The timed handler loop itself
took about 3–4 ms after the change. The fixture omits the earlier XPC peer and
Gate Client checks and the later target launch, so this is a handler measurement,
not an end-to-end `av inject` measurement. Temporary instrumentation existed only
in separately signed probe binaries and has been removed from the final source.

A live baseline ran 40 no-Secret `av inject` script executions with
`capabilities: {}`: median latency 402 ms, 17.15 s total wall time. The installed
app consumed 15.19 s CPU during that interval, but it also handled other automatic
operations. That CPU total cannot be attributed solely to the fixture. A
10-second `sample` capture repeatedly showed
`verifiedLauncherHelperAppSigningInfo` → `validateAppBundleResource` →
`SecStaticCodeCheckValidityWithErrors` on active request stacks.

## Verification

- Release Swift suite passed (322 tests reported; 32 entitlement-gated skips).
- CLI injection tests: eight passed. Secret custody boundary tests: three passed.
- Menu helper self-checks: 29 passed.
- New corruption regression rejects altered persisted content even when its UUID
  matches the expected record. It fails against the original implementation.
- Empty-ceiling checks cover inherited capabilities, requested Secrets,
  incompatible script execution, and a non-inject operation.
- Independent Standards and Spec/security reviews found no concrete issues in
  the final changes. The committed script checks verify branch eligibility;
  the temporary handler probe supplies the performance comparison.

Run the committed history benchmark alone to avoid CPU contamination:

```sh
AV_BENCHMARK_AUTHORIZATION=1 swift test --package-path src/menu-helper \
  -c release --disable-automatic-resolution --filter authorizationHistoryPerformance
cargo test --test inject_cli --test secret_custody_boundary
scripts/test-menu-helper-self-checks.sh src/menu-helper/.build/release/AutomicVaultMenubar
```

## Remaining work

Measure representative Secret-bearing requests with an isolated, properly
entitled app and synthetic Secrets. This session did not measure production
Keychain persistence or full Secret exchange before and after the changes.

Signature verification remains expensive. This session retains strict
all-architecture validation and exact helper resource validation; it adds no
cross-request identity cache. Further work must preserve the bounds in
[ADR 0033](adr/0033-targeted-app-launcher-validation.md).

Local raw measurements and temporary probes are in `/tmp/av-cpu-20260909`.
The installed app, production Secrets, and Authorization Policies were unchanged.
