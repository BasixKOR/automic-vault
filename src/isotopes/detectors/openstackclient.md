# openstackclient Radioisotope

`openstack` reads `~/.config/openstack/clouds.yaml` and optional
`~/.config/openstack/secure.yaml`, and those files can contain plaintext
passwords, tokens, and application credential secrets.

For a single configured cloud with simple `password`, `token`, or
`application_credential_secret` values, this radioisotope migrates the secret
values into the Automic Vault keychain as `OS_*` environment assignments,
blanks those fields on disk, and wraps `openstack` so the values are injected
only while the CLI runs.

More complex configs still use the older config-wrapper fallback: the original
default user OpenStack config files are stored in the keychain, supported
secret values are blanked on disk, and `openstack` reconstructs the original
files in a temporary config directory only while the CLI runs.

## Caveats

- We currently migrate only `~/.config/openstack/clouds.yaml` and
  `~/.config/openstack/secure.yaml`.
- The `OS_*` env path is limited to single-cloud configs with simple scalar
  secret values.
- Multi-cloud configs and complex YAML secret values fall back to temporary
  config reconstruction.
- Only password, token, and application credential secret fields are blanked
  on disk during migration.
- Project-local `./clouds.yaml` files and `/etc/openstack` are not migrated.
- Runtime config changes made while `openstack` runs are written to the
  temporary files and are not persisted back to keychain.
- Direct execution of the original binary will not receive credentials.
