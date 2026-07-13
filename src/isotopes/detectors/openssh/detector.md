# openssh Detector

## Trigger Conditions

- SSH private key is stored without passphrase encryption.

## Sensitive Files

- `~/.ssh/config`
- `~/.ssh/id_*`
- `identity files referenced by ~/.ssh/config`

## Why This is not Yet Hardened

SSH private keys are long-lived identity files used by `ssh`, agents, Git, and
other clients. Moving them would break those shared paths. Encrypt the key with
a passphrase and let Apple's OpenSSH integration store the passphrase in the
macOS Keychain.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
