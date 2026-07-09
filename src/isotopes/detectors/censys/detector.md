# censys Detector

## Trigger Conditions

- Censys config contains plaintext API credentials.

## Mitigation

```sh
sudo av harden censys
```

## Sensitive Files

- `~/.config/censys/censys.cfg`
