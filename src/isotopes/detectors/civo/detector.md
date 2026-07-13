# civo Detector

## Trigger Conditions

- civo config contains plaintext API keys.

## Mitigation

```sh
av harden civo
```

## Sensitive Files

- `$CIVO_CONFIG`
- `~/.civo.json`
