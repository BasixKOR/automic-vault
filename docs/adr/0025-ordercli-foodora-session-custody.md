# ADR 0025: Patch ordercli for Foodora session custody

Status: accepted

## Context

ordercli stores Foodora access and refresh tokens, a remotely discovered OAuth
client secret, pending MFA material, and session cookies in JSON config. Login,
token refresh, cookie import, MFA, and logout all mutate that shared state.
Upstream provides no credential helper, so a config wrapper would recreate
plaintext and could not safely cover the complete lifecycle. ordercli is a
single Go executable and can instead be patched at its config boundary.

## Decision

The ordercli Hardener installs an Automic Vault Isotope built from a pinned,
SHA-256-verified upstream source archive. The reviewed patch stores the complete
Foodora credential bundle under the fixed Secret Name
`ORDERCLI_FOODORA_SESSION`, writes only reserved `@av` markers to supported
config files, and routes reads, stores, and deletes through fixed ordercli-only
XPC operations. Conflicting configs, unknown Foodora fields, malformed bundles,
plaintext state after hardening, and partial markers fail closed.

The Hardener validates every current and legacy config before changing any,
stores one equal credential bundle with save-if-absent-or-equal semantics, and
then atomically replaces each affected mode-0600 file. Deliveroo metadata is not
credential-bearing and remains unchanged.

The release workflow builds the patched executable from the pinned upstream
commit with a pinned Go toolchain, signs it as identifier `ordercli` under
Automic Vault team `ZU76A67LGU` with Hardened Runtime and a trusted timestamp,
rejects embedded entitlements, and publishes it from this repository. The
privileged installer revalidates the archive digest, signature, runtime,
timestamp, and entitlements before installing `/usr/local/bin/ordercli`.

For every credential operation, the menu app derives the helper's live parent
from the kernel and binds the exact Target, Developer ID identity, Hardened
Runtime, process lifetime, complete arguments, provider scope, and Secret Name
into the Authorization Request. Those values and the credential shape are
revalidated immediately before Secret Application or mutation. Unknown
commands fail closed.

## Consequences

Supported Foodora session material remains under Automic Vault custody at rest
and is not recreated in config files. The authorized ordercli Target necessarily
receives the reusable bundle in memory; Automic Vault cannot make an authorized
or compromised Target keep it confidential after release. Custom plaintext
config files must be explicitly migrated before the patched Target will use
them. Upstream schema or authentication changes require a reviewed patch update.
