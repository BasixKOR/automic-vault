# vercel-cli Detector

## Trigger Conditions

- Vercel CLI auth config contains plaintext credentials.

## Sensitive Files

- `~/Library/Application Support/com.vercel.cli/auth.json`
- `~/.local/share/com.vercel.cli/auth.json`
- `~/.now/auth.json`
- `~/Library/Application Support/now/auth.json`
- `~/.local/share/now/auth.json`
- `~/.vercel/auth.json`
- `$XDG_DATA_HOME/com.vercel.cli/auth.json`
- `$XDG_DATA_HOME/now/auth.json`
