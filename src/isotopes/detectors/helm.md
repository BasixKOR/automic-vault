# helm Detector

## Trigger Conditions

- Helm repositories.yaml contains plaintext credentials.

## Sensitive Files

- `$HELM_REPOSITORY_CONFIG`
- `$HELM_CONFIG_HOME/repositories.yaml`
- `~/Library/Preferences/helm/repositories.yaml`
