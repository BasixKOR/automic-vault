# Cloudflare Wrangler Radioisotope

Detect-only coverage for Wrangler global auth files.

Wrangler can persist OAuth access and refresh tokens in its global config
directory. Because the CLI refreshes those tokens, this radioisotope reports
the plaintext state without attempting migration.
