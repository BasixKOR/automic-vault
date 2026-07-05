# terraform-core Radioisotope

Terraform stores Terraform Cloud/Enterprise API tokens in
`~/.terraform.d/credentials.tfrc.json`.

This radioisotope covers Homebrew core's `terraform` formula. It migrates that
credentials file into the Automic Vault keychain and wraps `terraform` so
`TF_CLI_CONFIG_FILE` points at a temporary non-secret config file that enables
Terraform's native credentials helper protocol. The helper itself is installed
as `~/.terraform.d/plugins/terraform-credentials-av` and delegates to
`av credential-helper terraform`.

## Caveats

- We currently migrate `credentials.tfrc.json` only.
- Existing `.terraformrc` settings are not merged into the temporary file.
- Direct execution of the original binary will not receive the helper config.
