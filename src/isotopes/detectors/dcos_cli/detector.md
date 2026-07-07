# dcos-cli Detector

## Trigger Conditions

- dcos-cli cluster config contains a plaintext ACS token.

## Sensitive Files

- `$DCOS_DIR/clusters/*/dcos.toml`
- `~/.dcos/clusters/*/dcos.toml`
