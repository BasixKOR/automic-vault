# gh-cli-hosts-token Detector

## Trigger Conditions

- GitHub CLI `hosts.yml` contains a non-empty `oauth_token` entry.

## Mitigation

```sh
av harden gh
```

## Sensitive Files

- `$GH_CONFIG_DIR/hosts.yml`
- `$XDG_CONFIG_HOME/gh/hosts.yml`
- `~/.config/gh/hosts.yml`
