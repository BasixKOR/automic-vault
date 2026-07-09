# gotify Detector

## Trigger Conditions

- Gotify config contains a plaintext application token.

## Mitigation

```sh
sudo av harden gotify
```

## Sensitive Files

- `$XDG_CONFIG_HOME/gotify/cli.json`
- `~/.gotify/cli.json`
