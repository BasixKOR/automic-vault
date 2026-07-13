# hcloud Detector

## Trigger Conditions

- hcloud config file contains plaintext API tokens.

## Mitigation

```sh
av harden hcloud
```

## Sensitive Files

- `$HCLOUD_CONFIG`
- `$XDG_CONFIG_HOME/hcloud/cli.toml`
- `~/.config/hcloud/cli.toml`
