# k6 Detector

## Trigger Conditions

- k6 config file contains a plaintext cloud token.

## Mitigation

```sh
sudo av harden k6
```

## Sensitive Files

- `~/Library/Application Support/k6/config.json`
- `~/.config/k6/config.json`
