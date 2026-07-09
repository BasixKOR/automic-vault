# node Detector

## Trigger Conditions

- npm user config contains a plaintext auth token.

## Mitigation

```sh
sudo av harden node
```

## Sensitive Files

- `$NPM_CONFIG_USERCONFIG`
- `~/.npmrc`
