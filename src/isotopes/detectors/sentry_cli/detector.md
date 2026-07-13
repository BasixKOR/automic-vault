# sentry-cli Detector

## Trigger Conditions

- Sentry CLI config contains a plaintext auth token.

## Mitigation

```sh
av harden sentry-cli
```

## Sensitive Files

- `~/.sentryclirc`
