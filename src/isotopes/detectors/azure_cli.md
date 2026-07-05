# azure-cli Detector

Reports when:
- Azure CLI MSAL token cache contains plaintext credentials.
- Azure CLI service principal cache contains plaintext credentials.
- Azure CLI legacy token cache contains plaintext credentials.

## Detection Caveats

- Scans `AZURE_CONFIG_DIR` when set, otherwise `~/.azure`.
