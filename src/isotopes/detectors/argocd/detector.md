# argocd Detector

## Trigger Conditions

- Argo CD config file contains plaintext tokens.

## Mitigation

```sh
sudo av harden argocd
```

## Sensitive Files

- `$ARGOCD_CONFIG_DIR/config`
- `~/.argocd/config`
- `$XDG_CONFIG_HOME/argocd/config`
- `~/.config/argocd/config`
