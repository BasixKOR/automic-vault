# doctl Detector

## Trigger Conditions

- doctl config contains plaintext DigitalOcean tokens.

## Mitigation

```sh
sudo av harden doctl
```

## Sensitive Files

- `$DIGITALOCEAN_CONFIG`
- `~/Library/Application Support/doctl/config.yaml`
- `$XDG_CONFIG_HOME/doctl/config.yaml`
- `~/.config/doctl/config.yaml`
