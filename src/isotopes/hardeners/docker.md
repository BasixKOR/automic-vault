# Docker

`av harden docker` keeps Docker Desktop's vendor-signed, Hardened Runtime CLI
and replaces its ambient credential helper with Automic Vault's Secret Gate.
It migrates credentials without printing them, installs a root-owned helper at
`/usr/local/bin/docker-credential-av` only when every containing directory is
root-owned and protected from group/world writes, and updates Docker's
`credsStore`.

The Docker helper protocol necessarily returns a usable registry token to an
authorized Docker process. Automic Vault verifies the live Docker Desktop code
signature, Hardened Runtime, process ancestry, arguments, and registry before
release. Non-Automic per-registry helpers and inline credentials fail closed.
