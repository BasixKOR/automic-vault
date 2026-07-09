# pnpm-auth-token Detector

## Trigger Conditions

- npm user config contains a plaintext auth token.

## Mitigation

```sh
sudo av harden pnpm
```

## Sensitive Files

- `$NPM_CONFIG_USERCONFIG`
- `~/.npmrc`
