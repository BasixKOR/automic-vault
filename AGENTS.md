- Commit at sensible intervals.

## Versioning

- `NUKE_BUILD_ID` is an automatic exact-build stamp. Do not manually bump it.
  Release and publish builds should derive it from the Git commit; local debug
  builds may use a stable local value to avoid unnecessary Rust rebuilds.
- `NUKE_PROTOCOL_VERSION` tracks the `av serve` protocol contract. Bump it when
  the GUI/helper protocol surface changes, including method names, request
  params, response payloads, required fields, error semantics depended on by the
  GUI, socket lifecycle, or daemon compatibility expectations.
- `NUKE_HELPER_VERSION` tracks the installed privileged helper. Bump it whenever
  privileged helper behavior changes, even if the XPC/protocol interface did not.
