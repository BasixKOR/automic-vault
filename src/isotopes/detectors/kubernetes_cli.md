# kubernetes-cli Radioisotope

`kubectl` reads kubeconfig files that commonly contain bearer tokens,
passwords, client key paths, or embedded client key data.

This radioisotope migrates the default kubeconfig into the Automic Vault
keychain and rewrites supported user entries to Kubernetes `exec` credential
plugins that call `av credential-helper kubernetes`. The wrapper provides a
short-lived approval token while `kubectl` is running.

## Caveats

- We currently migrate the default `~/.kube/config` file only.
- Multi-file `KUBECONFIG` setups must be migrated manually.
- Only bearer-token and embedded client-certificate user credentials are
  migrated to the exec helper.
- Passwords, auth-provider refresh credentials, and client-key file paths must
  be migrated manually.
- Kubeconfigs that rely entirely on exec auth plugins may not contain
  migratable secrets.
