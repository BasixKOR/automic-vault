# Homebrew Detector

## Trigger Conditions

- Homebrew exists at `/opt/homebrew/bin/brew` and the current user can modify
  `/opt/homebrew` or one of its immediate child directories.

## Rationale

Malware and agents can modify installed packages or Homebrew itself without
constraint.

After hardening, use `brew install` as usual; installed packages are owned by
`automic:vault`. Zsh users must initialize Homebrew through the hardened
launcher before `compinit` so zsh loads its user-owned completion mirror instead
of files inside the protected Homebrew prefix.

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

For zsh, replace any direct `/opt/homebrew/bin/brew shellenv` startup command
with these lines before `compinit`:

```zsh
eval "$(/usr/local/bin/brew shellenv zsh)"
autoload -Uz compinit
compinit
```

The launcher mirrors protected Homebrew completion files into
`~/.local/share/automic-vault/homebrew/zsh/site-functions` without `sudo` and
removes `/opt/homebrew/share/zsh/site-functions` from `fpath`.

## Sensitive Files

- `/opt/homebrew`
- `/opt/homebrew/*/`
