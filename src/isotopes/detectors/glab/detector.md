# glab Detector

## Trigger Conditions

- GLab config file contains plaintext tokens.

## Mitigation

```sh
sudo av harden glab
```

## Sensitive Files

- `$GLAB_CONFIG_DIR/config.yml`
- `$XDG_CONFIG_HOME/glab-cli/config.yml`
- `~/.config/glab-cli/config.yml`
- `~/Library/Application Support/glab-cli/config.yml`
