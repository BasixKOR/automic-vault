# dcos-cli Radioisotope

`dcos-cli` stores DC/OS cluster ACS tokens in plaintext TOML files under
`~/.dcos/clusters/*/dcos.toml`. The CLI supports `DCOS_DIR`, which gives the
radioisotope a narrow wrapper boundary.

The migration stores token-bearing `dcos.toml` files in the Automic Vault
keychain and removes `dcos_acs_token` lines from the original files. The
post-install wrapper copies the user's non-secret `~/.dcos` tree to a temporary
directory, overlays the keychain-backed token-bearing files, sets `DCOS_DIR`,
and then runs the original `dcos` launcher.

Implemented from the Homebrew popularity scan at formula version 1.2.0.
