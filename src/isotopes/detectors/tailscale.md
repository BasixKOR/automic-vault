# tailscale Detector

## Trigger Conditions

- Tailscale state file contains plaintext node identity state.

## Sensitive Files

- `/Library/Tailscale/tailscaled.state`
- `~/.local/share/tailscale/tailscaled.state`
- `/opt/homebrew/var/lib/tailscale/tailscaled.state`
- `/usr/local/var/lib/tailscale/tailscaled.state`
- `$XDG_DATA_HOME/tailscale/tailscaled.state`
