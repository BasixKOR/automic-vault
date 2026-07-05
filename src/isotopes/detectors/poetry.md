# poetry Detector

## Trigger Conditions

- Poetry auth.toml contains plaintext repository credentials.

## Sensitive Files

- `$XDG_CONFIG_HOME/pypoetry/auth.toml`
- `~/.config/pypoetry/auth.toml`
- `~/Library/Application Support/pypoetry/auth.toml`
- `~/Library/Preferences/pypoetry/auth.toml`
