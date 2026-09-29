# DigitalOcean CLI

`av harden doctl` installs the Automic Vault-signed repack of doctl 1.175.0
under `/opt/av/doctl/1.175.0-av.1`. Upstream executable code is unpatched.
The native AV launcher verifies the pinned binary and protected paths, then
applies `DIGITALOCEAN_ACCESS_TOKEN` directly to that Target without a shell.
The approval service rechecks installation before policy admission and Secret
release. The client rechecks after Approval and before execution.

The reviewed catalog contains 469 exact command paths and their aliases.
Help, local commands, unknown commands, plugins, `apps dev`, serverless support,
and the new harness runtime execute without the protected token. Explicit
credentials, alternate contexts, custom configuration/API endpoints and trace
logging also suppress AV token access. These commands may still use credentials
outside Automic Vault.

`auth token` discloses the token; `auth init` writes it to plaintext configuration.
Both require Secret Disclosure authorization. `compute droplet ssh` and registry
login may launch other executables and remain Unknown, requiring Approval.
Other operations without a reviewed policy classification also require Approval.

Migration retains the existing default-context-only boundary. Named-context
tokens require manual migration. Installation verification precedes credential
migration; a failed installation retains plaintext. Use `av scan` afterwards
for remaining credential sources and `av doctor doctl` for installation checks.

Updates require another reviewed signed repack and an AV release with new
archive and executable digests. Re-hardening repairs this pinned version. The
tap formula records the same artifacts; AV owns the protected runtime and launcher.
Code signing establishes identity and integrity, not destination safety or
confidentiality after Secret Application. This integration verifies the Target
before execution; it does not add a live credential-provider handshake.
