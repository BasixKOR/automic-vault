# Tailscale Radioisotope Detector

This detector reports readable plaintext Tailscale daemon state.

The Homebrew `tailscale` package installs both `tailscale` and `tailscaled`.
The sensitive identity state belongs to `tailscaled`, not the CLI. Upstream
macOS app builds can use Keychain-backed state, but the Homebrew/self-compiled
daemon path is treated as plaintext state.

Detected hazards:

- `/Library/Tailscale/tailscaled.state`
- `~/.local/share/tailscale/tailscaled.state`
- `$XDG_DATA_HOME/tailscale/tailscaled.state`
- Common Homebrew var state paths

This radioisotope is detect-only. A wrapper around `tailscale` would not secure
the daemon's persistent node identity. Remediation should happen in the daemon
state store or in a source isotope.
