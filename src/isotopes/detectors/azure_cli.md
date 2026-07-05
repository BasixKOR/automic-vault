# Azure CLI Radioisotope Detector

This detector reports plaintext Azure CLI credential caches.

Azure CLI uses MSAL for user authentication and stores token-cache and service
principal data under the Azure config directory. Current upstream code supports
encrypted persistence, including macOS Keychain persistence, but the default
encryption fallback is enabled only on Windows.

Detected hazards:

- `~/.azure/msal_token_cache.json`
- `~/.azure/service_principal_entries.json`
- Legacy `~/.azure/accessTokens.json`
- The same files under `$AZURE_CONFIG_DIR` when set

Encrypted `.bin` cache files are not reported.

This radioisotope is detect-only. The token cache is complex mutable MSAL
state, so a safe fix needs an upstream default/migration change or a source
isotope that patches the Azure CLI persistence layer.
