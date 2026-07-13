# cloudflare-wrangler Detector

## Trigger Conditions

- Wrangler auth config contains plaintext Cloudflare tokens.

## Sensitive Files

- `~/Library/Preferences/.wrangler/config/default.toml`
- `~/.wrangler/config/default.toml`
- `~/.config/.wrangler/config/default.toml`

## Why This is not Yet Hardened

Wrangler can persist OAuth access and refresh tokens in its global config
directory. Because the CLI refreshes those tokens, this detector reports the
plaintext state without attempting migration.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
