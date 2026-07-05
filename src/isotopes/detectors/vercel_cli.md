# Vercel CLI Radioisotope Detector

This detector reports plaintext Vercel CLI auth files.

Current Vercel CLI reads and writes `auth.json` in the global config directory.
That file can contain both an access token and a refresh token, and the CLI
updates it after token refreshes or reauthentication.

Detected hazards:

- `~/Library/Application Support/com.vercel.cli/auth.json`
- `~/.local/share/com.vercel.cli/auth.json`
- `~/.now/auth.json`
- `~/Library/Application Support/now/auth.json`
- `~/.local/share/now/auth.json`
- `~/.vercel/auth.json`

This radioisotope is detect-only. A normal wrapper would need to persist token
refresh writeback into the keychain, so the safer remediation is a source
isotope or an upstream keychain-backed auth store.
