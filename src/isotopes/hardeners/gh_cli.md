# GitHub CLI

## How Automic Vault Hardens `gh`

We provide a [patched version] of `gh` via our [tap]. The patches are concerned
with:

1. Is codesigned such that `gh` (and only `gh`) can access its
   secure credentials.
2. Ensures that authenticated `gh` usage goes via the Automic Vault Secret Gate
   system.

[patched version]: https://github.com/automic-vault/gh-cli
[tap]: https://github.com/automic-vault/homebrew-isotopes

## Credential Migration

Use `av harden gh` to migrate existing `gh` credentials into Automic Vault.

## Secret Gate

The menu bar app creates a `gh` Secret Gate as soon as the hardened CLI is
installed. Configure its default and per-app protection levels there. Read Only
auto-approves known read-only commands and `gh api` GET requests. Local Write
Access additionally approves `repo clone`, `pr checkout`, `gist clone`, and
download commands, which can change local files but do not mutate GitHub.
Trusted Access approves remote mutations, but still prompts for `gh auth token`
and `gh auth status --show-token`.

## Details

- The migration covers standard `hosts.yml` token entries and legacy macOS
  Keychain items named `gh:<host>`.
- Existing Git configuration can still delegate GitHub credentials to `gh auth
  git-credential`; the hardened `gh` helper path requests the token through
  Automic Vault.
- `av harden gh-cli` remains accepted as a compatibility alias.
