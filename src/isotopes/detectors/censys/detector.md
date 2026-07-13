# censys Detector

## Trigger Conditions

- Censys config contains plaintext API credentials.

## Mitigation

```sh
av harden censys
```

## Sensitive Files

- `~/.config/censys/censys.cfg`
