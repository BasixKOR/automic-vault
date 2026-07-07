# gh-cli-keychain-access Detector

## Trigger Conditions

- On macOS, a GitHub CLI Keychain item allows `/usr/bin/security` to read the secret without an interactive prompt.

## Sensitive Files

- Keychain generic-password items for GitHub CLI hosts.
