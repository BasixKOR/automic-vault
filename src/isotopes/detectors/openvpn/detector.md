# openvpn Detector

## Trigger Conditions

- OpenVPN profile contains inline plaintext key or password material.
- OpenVPN auth-user-pass file contains plaintext credentials.

## Sensitive Files

- `~/.openvpn/**`
- `$XDG_CONFIG_HOME/openvpn/**`
- `~/.config/openvpn/**`
- `~/Library/Application Support/OpenVPN/**`
- `~/Library/Application Support/Tunnelblick/Configurations/**`
- `auth-user-pass files referenced by scanned profiles`
