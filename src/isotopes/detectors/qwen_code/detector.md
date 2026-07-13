# qwen-code Detector

## Trigger Conditions

- Qwen Code settings contain plaintext API keys.

## Mitigation

```sh
av harden qwen-code
```

## Sensitive Files

- `~/.qwen/settings.json`
