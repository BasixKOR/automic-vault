# openhue-cli Detector

## Trigger Conditions

- OpenHue config contains a plaintext Hue application key.

## Sensitive Files

- `$XDG_CONFIG_HOME/openhue/config.yaml`
- `~/.openhue/config.yaml`

## Remediation

Run `av harden openhue-cli`. The hardener installs the signed OpenHue CLI
Isotope and migrates the Hue application key into Automic Vault custody while
leaving only bridge metadata and an `@av` marker on disk.
