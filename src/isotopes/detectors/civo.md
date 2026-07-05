# Civo CLI Radioisotope

`civo` stores API keys in `.civo.json` by default. It also supports
`CIVO_TOKEN`, which gives this radioisotope a narrow environment-wrapper
boundary.

This radioisotope migrates the active API key into the Automic Vault keychain,
removes plaintext API key material from the original config file, and injects
the key as `CIVO_TOKEN` only while `civo` runs. Non-secret config fields are
preserved in the original config file.

## Caveats

- If a token is written back into `.civo.json`, the detector will report it
  again.
- For named `apikeys` configs, the migrated token is selected from
  `current_apikey` when possible, otherwise from the first non-empty entry.
- Direct execution of the original binary will not receive credentials.
