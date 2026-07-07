# azure-cli Detector

## Trigger Conditions

- Azure CLI MSAL token cache contains plaintext credentials.
- Azure CLI service principal cache contains plaintext credentials.
- Azure CLI legacy token cache contains plaintext credentials.

## Sensitive Files

- `$AZURE_CONFIG_DIR/msal_token_cache.json`
- `$AZURE_CONFIG_DIR/service_principal_entries.json`
- `$AZURE_CONFIG_DIR/accessTokens.json`
- `~/.azure/msal_token_cache.json`
- `~/.azure/service_principal_entries.json`
- `~/.azure/accessTokens.json`
