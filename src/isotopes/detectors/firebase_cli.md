# firebase-cli Radioisotope

Firebase CLI stores login tokens in its configstore file.

This radioisotope migrates that configstore JSON into the Automic Vault
keychain and wraps `firebase` so `XDG_CONFIG_HOME` points at a temporary
configstore only while Firebase CLI runs.

## Caveats

- We currently migrate the default `firebase-tools.json` configstore file.
- Direct execution of the original binary will not receive the credentials.
