# ADR 0004: Separate Detection, Hardening, and Verification

- Status: Accepted
- Date: 2026-08-06

## Context

Environment checks, security transformations, and verification of installed protections answer different questions. Calling all three a scan or treating Hardened State as a general safety claim would overstate coverage.

## Decision

A Detector performs one read-only check for a supported Exposure or Hazard. A Scan runs the available Detectors at one point in time. A Finding reports the concrete condition. A clean Scan means that no supported Detector found an issue.

A Hardener is a Tool-specific transformation procedure. Hardening may migrate a Credential, configure a Tool, install a wrapper, or install an Isotope. Hardened State consists only of the invariants declared by that Hardener.

Doctor verifies the installed Automic Vault intervention, including identity, ownership, permissions, dependencies, and command resolution where applicable. Doctor does not replace environment detection and does not certify the Tool as safe.

An Isotope is an Automic Vault-compatible build or wrapper of a third-party Tool. It is a distribution artifact, not a Detector, Hardener, or Secret.

## Consequences

- Detector output must distinguish Exposure, Hazard, and evidence of Compromise.
- Scan results cannot claim complete security.
- Each Hardener must declare verifiable invariants.
- Doctor can find a bypass caused by command resolution, but cannot enforce system-wide execution mediation.
- Product docs should use Secret or Secret Name instead of the legacy phrase *isotope key*.
