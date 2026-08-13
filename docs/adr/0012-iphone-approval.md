# ADR 0012: Carry human Approval on iPhone

Status: accepted

## Context

Automic Vault's primary adversary includes agents with the user's normal Mac
privileges and computer-use access. A Mac-local Approval button lets such an
agent attempt to approve its own Authorization Request. A separate Apple device
can move the human action outside that input surface, but it must not move
Secret custody, policy evaluation, history, or enforcement away from the Local
Execution Boundary.

APNs does not prove that a notification was displayed, and iOS does not provide
a reliable persistent background connection. iPhone Mirroring and notification
forwarding can also project phone controls onto the Mac. The design therefore
cannot infer physical possession merely from delivery or from running an iPhone
app.

## Decision

Automic Vault provides opt-in iPhone Approval per Mac.

- A distinct 256-bit account root key synchronizes through iCloud Keychain.
  Devices that possess it join without a pairing ceremony.
- A Mac may enable the feature only after at least one iPhone has recently
  registered an APNs endpoint while proving possession of the root key.
- Once enabled, every human Approval moves to an iPhone. This includes requests
  to broaden durable authority. The Mac has no local allow fallback.
- Requests live until the originating Gate Client cancels. Relay or phone
  unavailability leaves them pending and fails closed.
- The Mac remains authoritative for pending requests and republishes them after
  reconnect. The relay does not persist request contents; it persists only
  opaque revoked room identifiers required for durable emergency recovery.
- Encrypted responses bind the complete immutable request. The first valid
  response wins; all later responses are stale.
- Routine, completely verified requests may be approved from an authenticated
  notification action. Higher-risk requests open the full app.
- Face ID or Touch ID is optional per iPhone. When enabled, Approval has no
  passcode or companion-device fallback. Denial never requires authentication.
- The app ships for iPhone only. Designed-for-iPhone-on-Mac distribution,
  Catalyst, iPad, and visionOS are excluded.
- Emergency Mac-local recovery requires system authentication and rotates the
  account key, invalidating all devices and Macs in the account.
- Secret values and durable Authorization History never leave the Mac.

The product warns that iPhone Mirroring, Show on Mac, and Apple Watch can expose
actionable notification surfaces when per-device biometrics are disabled. The
initial release does not claim to police the user's Apple-device configuration.

## Consequences

The Mac may accept a phone-carried Approval while its session is inactive or
its displays sleep, provided the feature is enabled, a phone registration
exists, the originating process remains alive, and every ordinary enforcement
check still succeeds. Mac-local AWS MFA entry retains the previous active-user
requirement.

Using an iCloud Keychain account as enrollment makes Apple account recovery,
device security, Mirroring configuration, and every device holding the root key
part of the Approval trust model. The initial release has account-wide recovery
rather than per-device revocation.

A relay or APNs outage can indefinitely delay work, but cannot grant authority.
This availability cost is intentional: silently restoring a Mac Approval button
would defeat the feature's purpose.
