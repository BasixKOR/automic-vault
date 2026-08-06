# Stripe CLI

## How Automic Vault Hardens `stripe`

Install the patched [Stripe CLI fork] from the Automic Vault Isotopes tap. On
macOS it stores and retrieves Stripe CLI credentials through the authenticated
Automic Vault XPC broker instead of Keychain or plaintext fallback files.

Credential reads use the Stripe Secret Gate, so the configured per-app policy,
human approval, and access audit apply to each use.

After installing the isotope, run `av harden stripe`. Existing API keys,
sessions, and user access tokens are moved from the `StripeCLI` Keychain
service or `credentials.json`; plaintext API keys in `config.toml` are
replaced with redacted markers only after the Vault writes succeed.

[Stripe CLI fork]: https://github.com/automic-vault/stripe-cli
