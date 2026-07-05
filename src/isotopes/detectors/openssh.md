# openssh Detector

## Trigger Conditions

- SSH private key is stored without passphrase encryption.

## Sensitive Files

- `~/.ssh/config`
- `~/.ssh/id_*`
- `identity files referenced by ~/.ssh/config`
