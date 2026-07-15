# Secret Gate security audit

Date: 2026-07-14

## Incident finding

The reported AWS commands did not reach the approval helper. The unified log has no XPC connection or access record for either command. A reused PID approval cannot explain that result: the transient key includes the PID, process start time, caller identity, exact operation, keys, target, arguments, working directory, flags, environment conflicts, script, and tool. Reuse also writes an access record before returning secrets.

The terminal process visible after the incident started at 19:09:27, 21 seconds after the 19:09:06 screenshot. Its later `which aws` result and environment do not describe the incident shell.

Two paths reproduce the observed no-XPC/no-log signature:

1. The installed CLI honored `AUTOMIC_VAULT_TEST_KEYCHAIN_DIR` outside tests. Pointing it at a directory containing a file named for a requested key skipped XPC and loaded that file. The exact installed `/usr/local/bin/av` reproduced this bypass. No surviving process or file proves that the incident used it.
2. Calling `/opt/homebrew/bin/aws` directly bypasses the PATH wrapper. It can succeed if the process has any ambient AWS provider, including temporary environment credentials, a credential process, SSO/login state, container credentials, or instance metadata. No surviving provider explains the incident.

The available evidence rules out the approval cache and confirms a helper bypass. It does not distinguish the two paths above or a shell-resolution variation that disappeared when the terminal restarted.

## Execution matrix

| Path or variation | Result | Change |
| --- | --- | --- |
| Missing, unreadable, or malformed gate policy | Previously fell back to permissive behavior | Fail closed to No Access |
| New gate defaults | Trusted Access is an explicit onboarding policy | Persist the explicit policy, including explicit No Access choices |
| Launcher identity unavailable | Resolver applied “All Other Apps” without knowing whether an override matched | Disable persistent auto-approval and require a prompt |
| Auto or manual approval | Reply could precede audit persistence | Persist and verify the record before returning secrets |
| Audit storage | Same-user processes could alter `UserDefaults` | Store production records in the app-private Data Protection Keychain |
| Transient PID approval | Includes process birth time and complete request identity | No bypass found; every reuse remains audited |
| Installed CLI test hooks | Release/copy could use test Keychain and path overrides | Accept test overrides only from debug binaries inside their Cargo profile |
| XPC server identity | Rust, GH, and Supabase clients checked an identifier only | Pin Apple anchor, team ID, and current app identifier |
| Keychain item group | Wildcard entitlement was first, so omitted `kSecAttrAccessGroup` stored items there | Write/query the private app-ID group and migrate verified legacy items |
| AWS profile selection | `aws-vault` and AWS CLI could use different profiles | Parse the command profile once and use it for both |
| AWS config/provider chain | Real AWS inherited HOME, config, credential process, SSO/login cache, metadata, and pager hooks | Run it with an empty HOME/config/credentials file, disabled metadata and pager, and scrubbed provider variables |
| AWS aliases and pager | Read-only argv could invoke a shell alias or external pager with credentials | Isolate config and force `--no-cli-pager` |
| AWS temporary files | Predictable PID-based pass path | Use a mode-0700 random directory and remove it on exit |
| AWS zsh startup | User `.zshenv` ran after long-term keys were injected | Invoke zsh with `-f`; use `/bin/sh` for the pass shim |
| GH/Supabase credential access | Signed clients request tokens over XPC and keep them in memory | No token environment inheritance found; peer verification tightened |
| Generic environment wrappers | `/bin/sh` exports approved values then execs the target | Shell startup is clean; the target and its children still receive the secret by design |
| Direct real executable | `/opt/.../bin/<tool>` remains callable without the wrapper | Architectural residual risk |

## Confirmed fixes

Automic Vault commits:

- `3482108` Fail closed when secret gate policy is unavailable
- `714e757` Record gate decisions before replying
- `0317818` Pin approval service to signing team
- `bd1661b` Isolate hardened AWS runtime
- `fddf4dd` Disable test keychain bypass in installed CLI
- `e9328d4` Fail closed when approval logging fails
- `4ac3fe2` Disable test overrides in installed CLI
- `5cd5b88` Migrate secrets to private keychain group
- `8d4a0d0` Skip shell startup files in AWS gate
- `4c3e734` Protect approval audit records in Keychain
- `e779532` Require launcher identity for auto approval

Isotope commits:

- GH CLI `d80395cbb` Pin approval service to signing team
- Supabase CLI `f2e0e5da` Pin approval service to signing team

## Residual risks

### Wrapper bypass

Secret Gates shadow commands on PATH; they do not mediate `exec`. A caller can run the underlying executable directly. Vault-managed credentials should remain unavailable, but any ambient provider can still authorize the direct process. Closing this boundary requires moving the real target behind an access-controlled launcher or adding execution mediation such as Endpoint Security. A doctor warning can detect PATH order but cannot prevent an absolute-path call.

### Generic Trusted Access

Generic wrappers inject a secret into the target process. The target can load plugins, invoke helpers, or expose its environment. Automic Vault cannot promise “all commands except secret dumps” for an arbitrary third-party CLI from argv classification alone. Trusted Access therefore means trusting that target's complete runtime, configuration, and child-process behavior. No Access or per-command approval is the defensible setting for an untrusted launcher.

### Keychain migration window

The migration build signs with the exact private group first and the legacy wildcard second. It copies each legacy item, verifies the bytes in the private group, then deletes the old item. A later release must remove `ZU76A67LGU.*` from the signed entitlement after users have run the migration build.

### Local audit scope

Keychain storage prevents ordinary same-user apps from rewriting the access list, and approved requests fail closed if the write cannot be verified. The list still retains only 50 records and is not a remote or append-only security log. Denied and failed requests remain best-effort because they never receive a secret.

### Deployment

The source fixes do not update `/usr/local/bin/aws`, `/usr/local/bin/av`, the installed app, GH, or Supabase. Rebuild and install all affected artifacts together. Mixing the new isotope clients with the old `com.automicvault.menu-helper` app identifier will fail closed at XPC peer verification.
