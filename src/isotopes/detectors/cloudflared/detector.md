# cloudflared Detector

## Trigger Conditions

- cloudflared certificate contains a plaintext private key.
- cloudflared tunnel credentials are stored in plaintext.

## Sensitive Files

- `~/.cloudflared/**`
- `$XDG_CONFIG_HOME/cloudflared/**`
- `~/.config/cloudflared/**`
