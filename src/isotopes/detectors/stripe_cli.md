# Stripe CLI Radioisotope Detector

This detector reports plaintext Stripe CLI API keys in `config.toml`.

Stripe CLI stores profile configuration under `~/.config/stripe/config.toml`
or `$XDG_CONFIG_HOME/stripe/config.toml`. Current upstream stores live-mode
keys in a keyring and writes redacted values to config, but test-mode and
legacy API key fields can still appear as plaintext.

This radioisotope is detect-only because safe remediation belongs in the
upstream profile/keyring store.
