# vault Detector

## Trigger Conditions

- Vault token helper file contains a plaintext token.

## Mitigation

```sh
av harden vault
```

## Sensitive Files

- `~/.vault-token`
