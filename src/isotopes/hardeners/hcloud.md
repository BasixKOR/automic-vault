# Hetzner Cloud CLI

`av harden hcloud` installs Hetzner's signed **hcloud 1.69.0** executable under
`/opt/av/hcloud/1.69.0` and replaces the old environment-wrapper launcher.
It preserves the vendor signature and pins the archive and executable digests
for arm64 and x86_64. Re-hardening repairs the pinned installation; adopting a
new release requires a command-surface and artifact review in Automic Vault.

The native AV launcher verifies the protected installation, applies the existing
positive command catalog, and injects `HCLOUD_TOKEN` directly into the verified
Target. The token never passes through a shell wrapper. The approval service
rechecks the pinned Target and protected paths before releasing the Secret;
the signed client verifies again after Approval and before execution.

The existing migration accepts one distinct token across supported hcloud
contexts. Multiple distinct tokens require manual migration. Installation
failure retains the credential file. Other credential sources remain outside
this migration; run `av scan` after hardening.

`context create --token-from-env` writes the token to plaintext configuration
and therefore requests Secret Disclosure. Sensitive config inspection also
requests Secret Disclosure. `server ssh` inherits the token into an external
SSH child and is Unknown, requiring Approval at every Access Level.
Unknown/future commands, help, ordinary local commands, and API commands with
endpoint overrides run without the protected token. Direct Secret Gate requests remain available.

Code signing verifies the installed Tool, not intent, destination configuration,
or confidentiality after Secret Application. The native Target is verified
before exec; this integration does not claim a live credential-provider handshake.
