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

## Read-Only Auto-Approval

The menu bar app can allow read-only `gh` commands without prompting. Enable
`Allow Read-Only gh Requests` from the hardened `gh` detail view. This only
auto-approves known read-only commands; raw API calls, token printing,
configuration, extensions, aliases, and unknown commands still require approval.

## Details

- The migration covers standard `hosts.yml` token entries and legacy macOS
  Keychain items named `gh:<host>`.
- Existing Git configuration can still delegate GitHub credentials to `gh auth
  git-credential`; the hardened `gh` helper path requests the token through
  Automic Vault.
- `av harden gh-cli` remains accepted as a compatibility alias.
