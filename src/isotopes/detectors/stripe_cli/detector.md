# stripe-cli Detector

## Trigger Conditions

- Stripe CLI config contains plaintext API keys.
- Stripe CLI Keychain credentials can be extracted non-interactively through
  `/usr/bin/security`.

## Sensitive Files

- `$XDG_CONFIG_HOME/stripe/config.toml`
- `~/.config/stripe/config.toml`

## Why This is not Yet Hardened

Stripe CLI owns and refreshes this profile state. Safe remediation belongs in
its upstream keyring store.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
