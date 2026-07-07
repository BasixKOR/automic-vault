# cloudflare-wrangler Detector

## Trigger Conditions

- Wrangler auth config contains plaintext Cloudflare tokens.

## Sensitive Files

- `~/Library/Preferences/.wrangler/config/default.toml`
- `~/.wrangler/config/default.toml`
- `~/.config/.wrangler/config/default.toml`
