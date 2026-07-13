# kubernetes-cli Detector

## Trigger Conditions

- kubeconfig contains plaintext cluster credentials.

## Sensitive Files

- `$KUBECONFIG`
- `~/.kube/config`

## Why This is not Yet Hardened

The retired hardener moved kubeconfig credentials to the macOS Keychain and
rewrote supported users to call `av credential-helper kubernetes`. The current
`av` CLI does not ship that credential-helper route. We need to review its
approval-token boundary before restoring a hardener that changes kubeconfig.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
