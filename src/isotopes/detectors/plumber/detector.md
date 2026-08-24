# plumber Detector

## Trigger Conditions

- Plumber local config exists outside Automic Vault custody.

## Sensitive Files

- `~/.batchsh/plumber.json`

## Remediation

Run `av harden plumber`. The hardener installs the signed Plumber Isotope and
migrates the complete local config into Automic Vault custody while leaving only
a fixed, non-secret marker in `~/.batchsh/plumber.json`.
