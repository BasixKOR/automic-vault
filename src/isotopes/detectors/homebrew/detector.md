# homebrew Detector

## Trigger Conditions

- Homebrew exists at `/opt/homebrew/bin/brew` and is not owned by `automic:vault`.
- `/usr/local/bin/brew` is missing or is not the Automic Vault setuid brew stub.

## Rationale

Malware and agents can modify installed packages or Homebrew itself without
constraint.

Our hardening changes nothing: `brew install` as usual, but the installed
packages are now owned by `automic:vault`.

We also add approval gates for sensitive actions like `brew upgrade` and
`brew install`.

## Sensitive Files

- `/opt/homebrew`
- `/opt/homebrew/bin/brew`
- `/usr/local/bin/brew`
