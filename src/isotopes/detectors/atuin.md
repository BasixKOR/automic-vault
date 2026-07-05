# Atuin Radioisotope

Detect-only coverage for Atuin sync state.

Atuin keeps the local sync encryption key and server session under the Atuin
data directory. Until Automic Vault has a write-safe Atuin integration, this
radioisotope reports those plaintext files without changing Atuin behavior.
