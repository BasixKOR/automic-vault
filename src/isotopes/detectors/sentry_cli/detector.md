# sentry-cli Detector

## Trigger Conditions

- Sentry CLI config contains a plaintext auth token.

## Mitigation

```sh
sudo av harden sentry-cli
```

## Sensitive Files

- `~/.sentryclirc`
