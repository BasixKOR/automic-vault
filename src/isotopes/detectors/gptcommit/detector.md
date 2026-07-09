# gptcommit Detector

## Trigger Conditions

- gptcommit global config contains a plaintext API key.
- gptcommit repository config contains a plaintext API key.

## Mitigation

```sh
sudo av harden gptcommit
```

## Sensitive Files

- `~/.config/gptcommit/config.toml`
- `./gptcommit.toml`
