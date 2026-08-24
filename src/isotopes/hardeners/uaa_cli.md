# uaa-cli Hardener

`av harden uaa-cli` installs the signed UAA CLI Isotope and migrates every
saved OAuth access and refresh token into one Automic Vault Secret. The UAA
config retains targets, contexts, expiry metadata, and reserved non-secret
`@av` markers.

The patched Target routes config reads, token updates, and context removal
through fixed UAA-only XPC operations. Each operation binds the live Target,
complete arguments, fixed credential scope, and exact Secret Name.

Unsupported fields, partial migration state, unsafe paths, malformed bundles,
and unsigned or incorrectly signed Targets fail closed.
