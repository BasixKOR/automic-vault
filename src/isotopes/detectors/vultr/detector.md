# vultr Detector

## Trigger Conditions

- vultr-cli config contains a plaintext API key.

## Mitigation

```sh
av harden vultr
```

## Sensitive Files

- `~/Library/Application Support/vultr-cli.yaml`
- `~/.vultr-cli.yaml`
