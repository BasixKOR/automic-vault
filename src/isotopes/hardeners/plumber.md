# plumber Hardener

`av harden plumber` installs the signed Plumber Isotope and migrates the
complete local config into one Automic Vault Secret. If the on-disk
`~/.batchsh/plumber.json` exists, it contains only a fixed, non-secret custody marker.

The patched Target routes local config reads and writes through dedicated
Plumber-only XPC operations. Each operation binds the live Target, complete
arguments, fixed local-config scope, and exact Secret Name. Plumber cluster-mode
KV storage is unchanged.

Invalid or oversized JSON, unsafe paths, unsigned or incorrectly signed Targets,
source drift, and unexpected archive contents fail closed.
