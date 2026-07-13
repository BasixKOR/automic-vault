# Homebrew Detector

## Trigger Conditions

- Homebrew exists at `/opt/homebrew/bin/brew` and the current user can modify
  `/opt/homebrew` or one of its immediate child directories.

## Rationale

Malware and agents can modify installed packages or Homebrew itself without
constraint.

Our hardening changes nothing about your workflow: `brew install` as usual, but
the installed packages are now owned by `automic:vault`.

We also add approval gates for sensitive actions like `brew upgrade` and
`brew install` so if an agent tries to sneakily install a package you can
vet its decision first.

## Caveats

This is not a supported configuration for Homebrew. However we use it and it
works fine for us. If you have issues with Homebrew while Automic Vault
Hardening is enabled please report the bug to *us* first.

The simple fact is: not hardening Homebrew can make everything else Automic
Vault does moot. If malware can just modify your hardened packages then the
hardening is either *much less effective* or potentially *completely useless*.

## Mitigation

```sh
sudo av harden brew
```

## Sensitive Files

- `/opt/homebrew`
- `/opt/homebrew/*/`
