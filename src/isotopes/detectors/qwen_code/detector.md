# qwen-code Detector

## Trigger Conditions

- Qwen Code settings contain plaintext API keys.

## Mitigation

```sh
sudo av harden qwen-code
```

## Sensitive Files

- `~/.qwen/settings.json`
