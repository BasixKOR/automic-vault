# OCI CLI Radioisotope

`oci setup config` writes OCI API configuration to `~/.oci/config`, including
references to private key files and optional token files. The radioisotope moves
the config and the referenced secret file contents into the macOS keychain, then
materializes temporary files while `oci` runs.

## Caveats

- Only the default config path and explicit `OCI_CLI_CONFIG_FILE` are migrated.
- Profiles must reference at most one private key file, security token file, and
  delegation token file.
- Session-token refreshes and setup commands write to temporary runtime files
  and are not persisted back to the keychain.
- Direct execution of the original binary will not receive credentials.
