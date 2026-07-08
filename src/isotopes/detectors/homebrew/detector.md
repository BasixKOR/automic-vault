# homebrew Detector

## Trigger Conditions

- Apple Silicon Homebrew exists at `/opt/homebrew/bin/brew`.
- `/usr/local/bin/brew` is missing or is not the Automic Vault setuid brew stub.
- `/opt/homebrew` is not owned by `automic:vault`.

## Sensitive Files

- `/opt/homebrew`
- `/opt/homebrew/bin/brew`
- `/usr/local/bin/brew`
