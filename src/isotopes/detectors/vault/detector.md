# vault Detector

## Trigger Conditions

- Vault token helper file contains a plaintext token.

## Mitigation

```sh
sudo av harden vault
```

## Sensitive Files

- `~/.vault-token`
