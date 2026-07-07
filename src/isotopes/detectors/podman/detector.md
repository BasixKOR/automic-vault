# podman Detector

## Trigger Conditions

- Podman registry auth file contains credentials.

## Sensitive Files

- `$REGISTRY_AUTH_FILE`
- `$XDG_RUNTIME_DIR/containers/auth.json`
- `$XDG_CONFIG_HOME/containers/auth.json`
- `~/.config/containers/auth.json`
