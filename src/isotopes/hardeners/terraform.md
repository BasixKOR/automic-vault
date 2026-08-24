# Terraform

`av harden terraform` installs HashiCorp's Developer-ID-signed, Hardened Runtime
Terraform Target and replaces plaintext host API tokens with the Automic Vault
credential helper. The helper binds each `get`, `store`, or `forget` request to
the live Terraform process and exact hostname. An active Homebrew formula,
competing CLI configuration, and `TF_TOKEN_*` credentials are removed or refused
rather than silently taking precedence.
