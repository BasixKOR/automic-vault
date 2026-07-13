# algolia Detector

## Trigger Conditions

- algolia config contains plaintext API keys.

## Mitigation

```sh
av harden algolia
```

## Sensitive Files

- `${XDG_CONFIG_HOME:-$HOME/.config}/algolia/config.toml`
