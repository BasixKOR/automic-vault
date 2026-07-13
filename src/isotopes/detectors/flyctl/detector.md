# flyctl Detector

## Trigger Conditions

- flyctl config file contains a plaintext access token.

## Mitigation

```sh
av harden flyctl
```

## Sensitive Files

- `~/.fly/config.yml`
