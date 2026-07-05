# Checkmarx AST CLI Radioisotope

`cx configure` can store Checkmarx One API keys or OAuth client secrets in
`~/.checkmarx/checkmarxcli.yaml`. The radioisotope moves those secrets into
the macOS keychain, removes `cx_apikey` and `cx_client_secret` from the
persisted config, and injects `CX_APIKEY` and `CX_CLIENT_SECRET` only while the
command runs.

Non-secret config such as base URIs, tenant, and client ID remains on disk.
The detector reports the config if the secret entries reappear.

## Caveats

- Only the default config path and explicit `CX_CONFIG_FILE_PATH` are migrated.
- The migration detects `cx_apikey` and `cx_client_secret` entries.
- Empty credential fields are not exported at runtime.
- Direct execution of the original binary will not receive credentials.
