# ADR 0031: Select Isotope installation by package availability and shape

Status: accepted

## Context

Automic Vault originally installed some Isotopes through Homebrew and forced
newer executable-only Isotopes into `/usr/local/bin`. Both paths consumed the
same signed tap manifest, release digest, and Developer ID identity, but the
different selection rules made Hardener behavior unpredictable.

Some verified upstream distributions are not executable-only. AWS CLI, for
example, includes a runtime and resources that must remain one versioned unit.
A generic direct installer for arbitrary multi-file tap packages does not yet
exist.

## Decision

When Homebrew is available and an Isotope is published in the Automic Vault
tap, the Hardener installs its fully qualified tap formula. New Isotope formula
names are distinct from their upstream formula names so Homebrew can keep the
upstream keg unlinked while installing the Isotope as a separate keg. If
Homebrew is not available, the Hardener may directly install an executable-only
Isotope into `/usr/local/bin`; an archive may contain more than one declared
executable but no undeclared payload. The direct installer verifies the exact
fork URL, SHA-256 digest, archive shape, Developer ID identity, Hardened Runtime,
timestamp, and entitlements, then records the digest in a protected receipt.

A verified multi-file vendor distribution uses a root-owned, versioned package
prefix under `/opt/av/<tool>` and exposes its command through `/usr/local/bin`,
as AWS CLI does. A non-executable-only Isotope from the tap has no direct
fallback yet and must fail closed when Homebrew is unavailable.

The static Secret Gate catalog records the selected Target path. Runtime checks
match that path, including normalized Homebrew Cellar paths, and independently
revalidate the live Target's signing identity and runtime protections.

## Consequences

Every tap Isotope follows one predictable Homebrew-first update path while
retaining a verified fallback on machines without Homebrew. Direct installation
does not grow a speculative package-layout format. Supporting a multi-file tap
Isotope without Homebrew requires a separate reviewed installer design.
