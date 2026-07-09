# travis Detector

## Trigger Conditions

- Travis CLI config contains a plaintext access token.

## Mitigation

```sh
sudo av harden travis
```

## Sensitive Files

- `~/.travis/config.yml`
