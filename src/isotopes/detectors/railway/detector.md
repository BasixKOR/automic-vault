# railway Detector

## Trigger Conditions

- Railway CLI auth state is stored in plaintext config.

## Sensitive Files

- `~/.railway/config.json`
- `~/.railway/config-staging.json`
- `~/.railway/config-dev.json`

## Why This is not Yet Hardened

The retired `railway` hardener moved the detected secret to the macOS Keychain,
then recreated `~/.railway/config.json` inside a temporary directory for each
run. We no longer consider a temporary plaintext file a sufficient security
boundary, so this detector remains report-only.

If a narrow environment-variable or credential-helper interface can cover this
state without writing the secret back to disk, we can reconsider the hardener.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
