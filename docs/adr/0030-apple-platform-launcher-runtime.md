# ADR 0030: Treat Apple platform Launchers as runtime protected

Status: accepted

## Context

Apple apps signed as part of a macOS release carry a platform identifier but
may omit the CodeDirectory Hardened Runtime flag. macOS intrinsically applies
the corresponding runtime protections to these platform binaries. Requiring
the flag alone incorrectly made Apple Launchers such as Chess ineligible for
Authorization Gate policy.

## Decision

Launcher runtime classification treats a valid signing-information platform
identifier as equivalent to the Hardened Runtime flag. The same blocked-runtime
entitlement checks still apply. Static app selection, live process inspection,
and Launcher Bundle inspection use this shared classification.

## Consequences

Apple platform apps can become Verified Launchers without a CodeDirectory
Hardened Runtime flag. A merely Apple-issued Developer ID signature is not
enough: the code must be identified by macOS as part of an operating-system
release, and normal signature, designated-requirement, entitlement, and live
process checks continue to fail closed.
