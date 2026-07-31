# openssh Detector

## Trigger Conditions

- SSH private key is stored without passphrase encryption.
- FIDO SSH key handle is stored without passphrase encryption (medium severity).

## Sensitive Files

- `~/.ssh/config`
- `~/.ssh/id_*`
- `identity files referenced by ~/.ssh/config`

## Why This is not Yet Hardened

SSH private keys are long-lived identity files used by `ssh`, agents, Git, and
other clients. Moving them would break those shared paths. Encrypt the key with
a passphrase and let Apple's OpenSSH integration store the passphrase in the
macOS Keychain.

FIDO `ecdsa-sk` and `ed25519-sk` files contain a key handle rather than the
hardware-bound signing key, so an unencrypted handle is reported at medium
severity instead of high.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
