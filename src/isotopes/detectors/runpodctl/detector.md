# runpodctl Detector

## Trigger Conditions

- runpodctl config contains a plaintext API key.

## Mitigation

```sh
av harden runpodctl
```

## Sensitive Files

- `~/.runpod/config.toml`
- `~/.runpod.yaml`
