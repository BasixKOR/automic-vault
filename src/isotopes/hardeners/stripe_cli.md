# Stripe CLI

## How Automic Vault Hardens `stripe`

`av harden stripe` installs the patched [Stripe CLI fork] from the Automic Vault
Isotopes tap when Homebrew is available. Without Homebrew it installs the same
signed release at `/usr/local/bin/stripe`; `av doctor stripe` reports direct
install updates. On macOS it stores and retrieves Stripe CLI credentials through
the authenticated Automic Vault XPC broker instead of Keychain or plaintext
fallback files.

Credential reads use the Stripe Secret Gate, so the configured per-Launcher
policy, Approval, and Authorization History apply to each use.

Existing API keys, sessions, and user access tokens are moved from the
`StripeCLI` Keychain service or `credentials.json`; plaintext API keys in
`config.toml` are replaced with redacted markers only after the Vault writes
succeed.

[Stripe CLI fork]: https://github.com/automic-vault/stripe-cli
