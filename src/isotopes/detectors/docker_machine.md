# Docker Machine Radioisotope

Detect-only coverage for Docker Machine TLS key material.

Docker Machine can leave host and client TLS private keys in
`~/.docker/machine`. This radioisotope reports unencrypted private keys without
modifying machine state.
