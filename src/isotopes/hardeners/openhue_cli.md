# openhue-cli Hardener

`av harden openhue-cli` installs the signed OpenHue CLI Isotope and migrates the
Hue application key into one Automic Vault Secret. The config retains the
bridge address, logging metadata, and a reserved non-secret `@av` marker.

The patched Target routes config reads and setup updates through fixed
OpenHue-only XPC operations. Each operation binds the live Target, complete
arguments, bridge scope, and exact Secret Name.

Unsupported YAML, duplicate fields, unsafe paths, invalid bridge scopes, and
unsigned or incorrectly signed Targets fail closed.
