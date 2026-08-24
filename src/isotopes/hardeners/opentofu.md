# OpenTofu

`av harden opentofu` uses an Automic Vault-signed, Hardened Runtime OpenTofu
Isotope because the upstream macOS executable is only ad-hoc signed. Terraform
and OpenTofu share the upstream credential-helper protocol and hostname-bound
Secret Values, but keep separate Target identities and Secret Gates.
