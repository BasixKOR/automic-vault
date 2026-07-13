# podman Detector

## Trigger Conditions

- Podman registry auth file contains credentials.

## Sensitive Files

- `$REGISTRY_AUTH_FILE`
- `$XDG_RUNTIME_DIR/containers/auth.json`
- `$XDG_CONFIG_HOME/containers/auth.json`
- `~/.config/containers/auth.json`

## Why This is not Yet Hardened

The retired `podman` hardener moved the detected secret to the macOS Keychain,
then recreated `$REGISTRY_AUTH_FILE` inside a temporary directory for each run.
We no longer consider a temporary plaintext file a sufficient security boundary,
so this detector remains report-only.

If a narrow environment-variable or credential-helper interface can cover this
state without writing the secret back to disk, we can reconsider the hardener.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
