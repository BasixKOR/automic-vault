# openhue-cli Detector

## Trigger Conditions

- OpenHue config contains a plaintext Hue application key.

## Sensitive Files

- `$XDG_CONFIG_HOME/openhue/config.yaml`
- `~/.openhue/config.yaml`

## Why This is not Yet Hardened

The retired `openhue-cli` hardener moved the detected secret to the macOS
Keychain, then recreated `$XDG_CONFIG_HOME/openhue/config.yaml` inside a
temporary directory for each run. We no longer consider a temporary plaintext
file a sufficient security boundary, so this detector remains report-only.

If a narrow environment-variable or credential-helper interface can cover this
state without writing the secret back to disk, we can reconsider the hardener.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
