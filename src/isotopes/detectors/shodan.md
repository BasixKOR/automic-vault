# Shodan Radioisotope

The Shodan CLI stores its API key in a local `api_key` file under
`~/.shodan` or `~/.config/shodan`. The radioisotope migrates that key into the
Automic Vault keychain and exposes it through a temporary config directory only
while `shodan` runs.

The wrapper does not migrate API keys supplied through custom user workflows,
and direct execution of the original binary will not receive the migrated key.
