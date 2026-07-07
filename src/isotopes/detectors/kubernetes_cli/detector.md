# kubernetes-cli Detector

## Trigger Conditions

- kubeconfig contains plaintext cluster credentials.

## Sensitive Files

- `$KUBECONFIG`
- `~/.kube/config`
