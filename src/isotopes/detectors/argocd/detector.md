# argocd Detector

## Trigger Conditions

- Argo CD config file contains plaintext tokens.

## Sensitive Files

- `$ARGOCD_CONFIG_DIR/config`
- `~/.argocd/config`
- `$XDG_CONFIG_HOME/argocd/config`
- `~/.config/argocd/config`
