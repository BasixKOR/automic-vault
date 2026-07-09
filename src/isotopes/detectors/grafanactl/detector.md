# grafanactl Detector

## Trigger Conditions

- grafanactl config contains plaintext credentials.

## Mitigation

```sh
sudo av harden grafanactl
```

## Sensitive Files

- `$XDG_CONFIG_HOME/grafanactl/config.yaml`
- `~/.config/grafanactl/config.yaml`
