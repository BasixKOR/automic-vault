# fastly radioisotope

The Fastly CLI stores API and SSO credentials in the user config file at
`~/Library/Application Support/fastly/config.toml` on macOS, or in the
platform fallback `~/.fastly/config.toml`.

This radioisotope migrates that config file into Automic Vault and replaces the
plaintext file with a migration marker. The wrapped `fastly` launcher restores
the config into a temporary private directory only for the duration of the
command.

The wrapper also exposes `FASTLY_API_TOKEN` from Automic Vault for commands
that use Fastly's environment-token path.

