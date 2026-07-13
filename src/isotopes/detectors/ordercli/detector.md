# ordercli Detector

## Trigger Conditions

- ordercli session state is stored in plaintext config.

## Sensitive Files

- `~/Library/Application Support/ordercli/config.json`
- `~/Library/Application Support/foodcli/config.json`
- `~/Library/Application Support/foodoracli/config.json`

## Why This is not Yet Hardened

The retired `ordercli` hardener moved the detected secret to the macOS Keychain,
then recreated `~/Library/Application Support/ordercli/config.json` inside a
temporary directory for each run. We no longer consider a temporary plaintext
file a sufficient security boundary, so this detector remains report-only.

If a narrow environment-variable or credential-helper interface can cover this
state without writing the secret back to disk, we can reconsider the hardener.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
