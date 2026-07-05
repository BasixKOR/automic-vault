## What It Does

`av harden gh-cli` verifies that the Automic Vault `gh-cli` isotope is installed at `/opt/homebrew/opt/gh-cli/bin/gh`, then tells you to run `gh auth av-migrate`.

This repository does not perform the GitHub credential migration itself. The hardener is a handoff to the isotope-provided `gh` command and its `gh auth av-migrate` subcommand.

## How It Protects You

The intended protection is to migrate GitHub CLI authentication away from the default `gh` credential storage path and into Automic Vault-managed storage, so GitHub tokens are not left available through the normal GitHub CLI credential helper surface.

## Caveats

- The migration logic lives in the installed `gh-cli` isotope, not in this Rust module.
- The hardener only checks for `/opt/homebrew/opt/gh-cli/bin/gh` and prints the migration command.
- You still need to run `gh auth av-migrate` after `av harden gh-cli`.
- Existing Git configuration can still delegate GitHub credentials to `gh auth git-credential`; the Git detector covers that exposure separately.
