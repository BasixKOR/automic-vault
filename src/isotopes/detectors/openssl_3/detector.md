# openssl@3 Detector

## Trigger Conditions

- OpenSSL private key is stored without passphrase encryption.

## Sensitive Files

- `~/.ssl/**`
- `~/.certs/**`
- `~/certs/**`
- `~/.config/openssl/**`
