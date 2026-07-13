# pnpm-auth-token Detector

## Trigger Conditions

- npm user config contains a plaintext auth token.

## Mitigation

```sh
av harden pnpm
```

## Sensitive Files

- `$NPM_CONFIG_USERCONFIG`
- `~/.npmrc`
