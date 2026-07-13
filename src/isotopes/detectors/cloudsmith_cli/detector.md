# cloudsmith-cli Detector

## Trigger Conditions

- cloudsmith credentials contain a plaintext API key.

## Mitigation

```sh
av harden cloudsmith-cli
```

## Sensitive Files

- `~/Library/Application Support/cloudsmith/credentials.ini`
- `~/.cloudsmith/credentials.ini`
