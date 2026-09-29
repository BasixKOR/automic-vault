# ADR 0060: Install a signed doctl repack behind native Secret routing

Status: accepted

## Context

The doctl environment wrapper passed a protected token through a shell to a
mutable package-manager executable. DigitalOcean's 1.175.0 macOS releases are
ad-hoc signed, so they cannot supply the required publisher identity. The CLI
already accepts `DIGITALOCEAN_ACCESS_TOKEN`; a credential patch is unnecessary.

## Decision

Repack the exact upstream arm64 and amd64 executables with Automic Vault's
Developer ID (`ZU76A67LGU`), identifier `doctl`, Hardened Runtime, no entitlements
and a secure timestamp. The packaging-only fork retains upstream licensing and
pins input digests. No upstream executable code is patched. Both reviewed
architectures link only Apple/system libraries. Signing does not assert a
separate notarization assessment.

Publish the two archives on the maintained fork as `v1.175.0-av.1`. Both the AV
Hardener and the tap formula pin those output hashes. AV additionally pins
whole-executable hashes, binding the reviewed code, signing flags, entitlements
and dependencies. Re-signing produces new artifacts and requires fresh pins.

The Hardener owns `/opt/av/doctl/1.175.0-av.1/doctl` and the native AV launcher
at `/usr/local/bin/doctl`. This is an explicit exception to default Homebrew
installation: the pre-execution environment-injection boundary requires
root-protected ancestry, not a mutable Homebrew target. The installer verifies
all ancestors before staging, copies the bounded archive with no symlink
following, rehashes it, streams only the executable from an unprivileged tar,
and verifies hash and signature before atomic installation. Reject symlinks,
hard links, ACLs, writable ancestry and unmanaged launcher collisions. Failed
activation restores the prior managed launcher; an unused verified generation
may remain. Root compromise is outside the boundary.

The native launcher requests the default-context Secret only for the reviewed
positive command catalog. Secret Application targets the exact native binary,
without a shell. The Gate binds `inject`, Target path, Secret Name and AV caller.
The approval service checks the installation at policy admission, preparation
and immediately before delivery after authorization recording. The client
rechecks after Approval. This establishes pre-execution artifact identity,
not a live provider handshake. Remove the old static wrapper route and reject
old installed wrapper execution in updated clients.

## Upstream command review

Reviewed `digitalocean/doctl` v1.175.0, commit
`776faec72dd6e13556f37340f068fd76b16ad575`. The packaging fork's `av/catalog.go`
emits the 469 exact runnable paths and aliases in `doctl-commands.txt` from the
Cobra tree. API command runners initialize the credential client through
`commands/command.go` and `commands/command_config.go`. Do not combine leaf
names with arbitrary parent groups. Unrecognized paths remain tokenless.

Exclude local auth management, local spec validation, `apps dev`, plugin
execution, all serverless support and the new harness runtime. The latter
families may execute untrusted local code or mutable Node support. Help,
malformed/non-UTF-8 arguments, alternative credentials/contexts, custom config
or API endpoints and trace logging suppress protected-token requests.

`commands/auth.go` shows that `auth token` prints the token and `auth init`
persists it to config. Both are Secret Disclosure. Droplet SSH and registry
login may execute external programs; classify them Unknown. Do not infer
read-only effects from a command name elsewhere in the catalog. Existing
reviewed policy rows remain narrow; other routed commands require Approval.
The Target still controls its memory, configuration, proxies, children and
output after Secret Application.

## Migration and updates

Retain the default-context-only migration and refusal of named-context tokens.
Verify installation before accessing migration; failed installation retains
plaintext. Doctor checks pinned content, signature, ancestry and launcher.
Updates require an explicitly reviewed AV release; no latest-version lookup or
runtime fallback to a system doctl is permitted. Draft assets and the tap PR
must be published/merged before this Hardener ships. Signed end-to-end delivery
and privileged installation remain release-validation requirements.
