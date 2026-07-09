# checkov Detector

## Trigger Conditions

- Checkov API key is stored in plaintext credentials file.

## Mitigation

```sh
sudo av harden checkov
```

## Sensitive Files

- `~/.bridgecrew/credentials`
