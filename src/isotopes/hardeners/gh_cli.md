# GitHub CLI

`av harden gh` verifies that the Automic Vault `gh` isotope is installed, imports legacy GitHub CLI tokens into Automic Vault storage, removes plaintext `oauth_token` entries from `hosts.yml`, and deletes old `gh:*` Keychain items when present.

The hardened `gh` isotope asks the Automic Vault XPC helper for tokens only when GitHub CLI actually needs a token. The helper receives request metadata such as the target binary, arguments, working directory, requested key name, and a human-readable detail string.

## Caveats

- `av harden gh-cli` remains accepted as a compatibility alias.
- The migration covers standard `hosts.yml` token entries and legacy macOS Keychain items named `gh:<host>`.
- Existing Git configuration can still delegate GitHub credentials to `gh auth git-credential`; the hardened `gh` helper path requests the token through Automic Vault.
