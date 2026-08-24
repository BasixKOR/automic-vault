# uaa-cli Detector

## Trigger Conditions

- UAA CLI config contains plaintext OAuth tokens.

## Sensitive Files

- `~/.uaa/config.json`

## Remediation

Run `av harden uaa-cli`. The hardener installs the signed UAA CLI Isotope and
migrates saved OAuth tokens into Automic Vault custody while leaving only
non-secret context metadata and `@av` markers on disk.
