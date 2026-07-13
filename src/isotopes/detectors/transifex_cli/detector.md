# transifex-cli Detector

## Trigger Conditions

- Transifex root config contains plaintext credentials.

## Mitigation

```sh
av harden transifex-cli
```

## Sensitive Files

- `~/.transifexrc`
