# ADR 0059: Verify the vendor hcloud Target before environment application

Status: accepted

## Context

The hcloud environment wrapper selected a same-user-mutable executable and
passed `HCLOUD_TOKEN` through `/bin/sh`. Positive command routing did not
establish the credential-bearing Target's integrity.

Hetzner publishes eligible native executables. Reviewed release **1.69.0**,
source commit `88350084112192df1317e3cc101419ddcb85c689`, has Developer ID
Application identity `hcloud`, team `4PM38G6W5R`, Hardened Runtime, no
entitlements and only Apple/system dynamic dependencies on both arm64 and
x86_64. Both downloaded executables pass strict codesign verification.
This assessment does not assert a separate notarization check.

## Decision

Preserve the vendor artifact under `/opt/av/hcloud/1.69.0/hcloud`. Pin both
archive and executable SHA-256 digests in AV. The privileged installer copies
and hashes the archive in protected staging, extracts only the executable to a
bounded stream with an unprivileged tar process, verifies the exact digest and
vendor signature, and atomically installs the binary and launcher. No upstream
installer or package-manager post-install code runs. Reject symlinks, unexpected
ownership/modes, writable ancestors, hard-linked files and ACLs. Refuse to
replace an unmanaged `/usr/local/bin/hcloud` command.

The root-owned launcher invokes the signed AV CLI's native hcloud entry point.
It verifies installation and applies the existing positive routing catalog.
Only reviewed credential-consuming commands request `HCLOUD_TOKEN`. All other
invocations remove that variable and execute without AV credential access.
The native client injects directly into the pinned executable, without a shell.

The hcloud Secret Gate route binds `inject`, the exact versioned native Target,
`HCLOUD_TOKEN`, the AV Gate Client and injection options. The approval service
verifies the protected installation before policy admission, after Approval
when preparing Secrets, and immediately before release after recording. The
client repeats verification after the decision and before exec. Target identity
here is the verified pre-execution artifact, not a live credential-provider
handshake. Root-protected ancestry closes the same-user replacement window;
root compromise remains outside the threat model.

The old shell route is removed from the static Gate catalog. Updated clients
refuse the old installed wrapper and Doctor requests re-hardening. Direct Secret
Gate requests retain their independent authority model. No catalog-wide change
to Hardened State is implied for other environment wrappers.

## Command review and limits

Review the upstream `internal/cli/root.go`, `internal/cmd`,
`internal/state/config` and `internal/state/state.go` at the pinned commit.
Existing API command paths remain positively enumerated. New `api` commands
remain tokenless pending a separate operation review. Help, completion, local
configuration, unknown commands, malformed/non-UTF-8 routing and explicit
endpoint overrides cannot obtain the protected token through the launcher.

`context create --token-from-env` persists the token in plaintext and is Secret
Disclosure, as is sensitive configuration inspection. `server ssh` and its
plural alias can execute an external SSH command with the token inherited;
classify these as Unknown, requiring Approval at every Access Level. This is
not containment of SSH, project code, endpoints, proxy configuration or output
once the Target receives a Secret.

Retain the existing one-distinct-token migration and its unsupported-context
limits. Complete installation verification precedes migration. Failure to
install retains plaintext; a failed launcher verification restores the prior
managed launcher. A failed installation may leave an unused verified binary.
Re-hardening repairs the pinned version; updates require another reviewed AV
release with new digests and command coverage. Keep older versioned artifacts
until no process relies on them. No automatic vendor-version selection occurs.
