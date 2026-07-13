# twine Detector

## Trigger Conditions

- Twine config contains plaintext package index credentials.

## Mitigation

```sh
av harden twine
```

## Sensitive Files

- `~/.pypirc`
