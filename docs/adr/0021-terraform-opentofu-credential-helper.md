# ADR 0021: Bind Terraform registry credentials to signed native Targets

Status: accepted

## Context

Terraform and OpenTofu store registry and HCP Terraform tokens in
`~/.terraform.d/credentials.tfrc.json`. Both Tools support the same external
credentials-helper protocol, but that protocol returns a reusable token to any
process that can invoke the helper for a hostname. The protocol itself provides
neither process authentication nor operation authorization.

HashiCorp's native macOS Terraform release has a Developer ID Application
signature and Hardened Runtime. The upstream OpenTofu macOS executable is
ad-hoc signed, so it cannot provide an equivalent live Target identity.

## Decision

The Terraform and OpenTofu Hardeners install an exact
`~/.terraform.d/plugins/terraform-credentials-av` launcher for the signed
Automic Vault CLI. They migrate only canonical hostname entries containing
exactly one nonempty `token`, store each credential as an opaque Secret, remove
the plaintext `credentials` object, and configure only the `av` credential
helper. A Secret Name is the `TERRAFORM_HOST_CREDENTIAL_` prefix plus the
uppercase SHA-256 digest of the canonical hostname.

The Hardener refuses `TF_CLI_CONFIG_FILE`, `TERRAFORM_CONFIG`, `TF_TOKEN_*`, a
nonempty `~/.terraformrc`, a different credentials helper, malformed credential
objects, and noncanonical hostnames. It stores every Secret with
save-if-absent-or-equal semantics before atomically replacing the configuration.
Failure leaves the original plaintext configuration available for recovery; it
does not delete the only copy of a credential.

Terraform is downloaded from HashiCorp's HTTPS release service. The privileged
installer accepts only the expected archive entries, extracts only the
`terraform` executable without running package scripts, and requires identifier
`terraform`, HashiCorp team `D38WU7D763`, Developer ID Application, Hardened
Runtime, a trusted timestamp, and no embedded entitlements before installing the
root-owned Target at `/usr/local/bin/terraform`.

The reviewed `automic-vault/opentofu` fork release pins an OpenTofu version,
verifies OpenTofu's signed checksum manifest with its release-workflow Sigstore
identity, extracts only the expected executable, and signs it with identifier
`tofu`, Automic Vault team `ZU76A67LGU`, Hardened Runtime, and a trusted
timestamp. It also rejects embedded entitlements. The signed Isotopes tap pins
the exact fork release URL and digest. The Hardener installs that formula when
Homebrew is available, or verifies the manifest, digest, and Automic Vault
signature before installing the root-owned Target at `/usr/local/bin/tofu`
otherwise, as specified by
[ADR 0031](0031-isotope-installation-selection.md).

The Terraform Hardener unlinks an active upstream Homebrew formula before using
its verified vendor Target. The OpenTofu Hardener replaces an upstream formula
with the Isotopes tap formula when Homebrew is available. Both verify command
resolution before moving Secrets; a shadowing Target is not Hardened State.

For `get`, `store`, and `forget`, the menu app obtains the helper's live parent
from the kernel. It accepts only the exact Target path, signing identifier, team,
Developer ID signature, Hardened Runtime, and process lifetime for the selected
Tool. The hostname, derived Secret Name, Target, arguments, and parent identity
are bound into the complete Authorization Request and revalidated immediately
before Secret Application or mutation. Unknown commands fail closed in the
Authorization Gate classifier.

## Consequences

Terraform and OpenTofu can share a hostname credential because they implement
the same protocol, while their separate Authorization Gates retain independent
Target identity and policy. An arbitrary same-user process cannot retrieve,
replace, or delete a credential without a live eligible parent Target and an
Authorization Decision.

The authorized Target necessarily receives the reusable token in memory. The
credential-helper protocol has no operation-scoped capability, and Automic
Vault cannot make an authorized or compromised Target keep the token
confidential after release.

Automic Vault assumes responsibility for the installed Targets and their update
path. Changed signatures, unsafe runtime exceptions, shadowing commands,
unknown operations, competing credential sources, and incomplete migrations
fail closed.
