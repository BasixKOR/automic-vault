# Ansible Radioisotope

This radioisotope protects Ansible Galaxy API tokens used by
`ansible-galaxy`.

Ansible's Galaxy client reads a YAML token file at
`~/.ansible/galaxy_token` by default. The file contains a `token:` value used
for role and collection publishing or authenticated Galaxy requests.

Automic Vault migrates the supported token to the macOS keychain, clears the
plaintext file, and wraps `ansible-galaxy` so it receives a temporary
`ANSIBLE_GALAXY_TOKEN_PATH` file only for the approved command invocation.

Detected and migrated:

- `~/.ansible/galaxy_token`
- The explicit `ANSIBLE_GALAXY_TOKEN_PATH` file when that environment variable
  is set during migration or scanning

Not migrated:

- Per-server Galaxy credentials in `ansible.cfg`
- Tokens supplied on the command line
- Inventory, vault, SSH, cloud, or module-specific secrets

Those are caller/project inputs rather than Ansible Galaxy's package-owned
token store.
