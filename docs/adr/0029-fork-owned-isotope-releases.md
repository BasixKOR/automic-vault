# ADR 0029: Bind Isotope updates to Tool fork releases

Status: accepted

Installation selection was refined by
[ADR 0031](0031-isotope-installation-selection.md).

## Context

Automic Vault maintains a separate GitHub fork for each patched Tool. Those
forks publish signed `cli-<version>.tgz` Isotope releases on the Tool's own
version cadence. Publishing duplicate Isotope archives with the Automic Vault
app coupled Tool updates to an unrelated app version and left the Hardener
looking for assets that were not present in the app release.

The Isotopes Homebrew tap already pins each exact fork release URL and SHA-256
digest. The existing installer also verifies the extracted Target's Developer
ID identity, Hardened Runtime, trusted timestamp, and lack of entitlements.

## Decision

The signed Isotopes Homebrew formula is the update manifest for each Isotope.
The Hardener accepts a formula only when it contains one URL under the exact
expected `automic-vault` Tool fork and one valid SHA-256 digest. Homebrew
installs the formula when it is available. Otherwise executable-only Isotopes
are downloaded from that URL, installed as root-owned Targets under
`/usr/local/bin`, and bound to the digest with a protected receipt.

The Automic Vault app release contains only app-owned artifacts. It does not
build, attest, checksum, or publish duplicate Tool Isotopes.

## Consequences

Each Isotope can update with its upstream Tool while the app and Hardener use
one manifest format and one verification path. A changed formula cannot select
a different organization or fork, and a matching archive must still satisfy
the pinned digest and Automic Vault code-signing requirements before it can
replace an installed Target. Formula or release unavailability fails closed.
