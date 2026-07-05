# cloudflared Radioisotope

Detect-only coverage for Cloudflare Tunnel credentials.

cloudflared tunnel state can include certificate private keys and tunnel
credential JSON files. These are service credentials, so this radioisotope
reports bounded user-level exposures without changing tunnel configuration.
