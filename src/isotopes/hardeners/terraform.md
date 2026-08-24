# Terraform

`av harden terraform` keeps HashiCorp's Developer-ID-signed, Hardened Runtime
Terraform Target and replaces plaintext host API tokens with the Automic Vault
credential helper. The helper binds each `get`, `store`, or `forget` request to
the live Terraform process and exact hostname. Competing CLI configuration and
`TF_TOKEN_*` credentials are refused rather than silently taking precedence.
