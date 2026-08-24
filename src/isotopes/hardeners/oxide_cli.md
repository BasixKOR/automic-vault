# Oxide CLI Hardener

`av harden oxide-cli` installs the signed Oxide CLI Isotope and migrates every
supported profile token from `~/.config/oxide/credentials.toml` into Automic
Vault. The file retains only the profile, host, user, token identifier,
expiration metadata, and the non-secret `@av` marker.

The patched Target rejects plaintext and `OXIDE_TOKEN` credentials. Token
reads, login stores, and logout deletes use fixed Oxide-only operations through
the signed `av` Gate Client. Each operation binds the live Oxide Target,
complete arguments, working directory, profile, host, and derived Secret Name.

Unknown credential fields fail hardening without changing the file. The
Hardened State covers the patched `/usr/local/bin/oxide` Command; code signing
does not establish user intent or protect a token after the Target receives it.
