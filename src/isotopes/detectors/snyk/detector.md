# snyk Detector

## Trigger Conditions

- Snyk CLI configstore contains credentials.

## Mitigation

```sh
av harden snyk
```

## Sensitive Files

- `$XDG_CONFIG_HOME/configstore/snyk.json`
- `~/.config/configstore/snyk.json`
