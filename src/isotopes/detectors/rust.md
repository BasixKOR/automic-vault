# Rust Radioisotope

This radioisotope modifies the Homebrew `rust` package, but only changes the
installed `cargo` launcher. `rustc`, `rustdoc`, `rustfmt`, and other Rust tools
continue to run without isotope credential injection.

## Security Model

Plaintext crates.io publishing tokens are commonly stored in
`~/.cargo/credentials.toml`, or in the legacy `~/.cargo/credentials` file, under
the default `[registry]` table. The migration stores one token in the Automic
Vault isotope keychain as `CARGO_REGISTRY_TOKEN` and removes the plaintext token
line from the Cargo credentials file.

The post-install hook wraps `/opt/rust/bin/cargo`. The wrapper enables
Cargo's native credential provider protocol for `cargo login`, `logout`,
`publish`, `yank`, and `owner`, delegating token access to
`av credential-helper cargo` while the original Cargo launcher runs.

## Caveats

- Only the default crates.io registry token is supported.
- Custom registry tokens under `[registries.<name>]` are not migrated.
- Cargo subcommands other than `login`, `logout`, `publish`, `yank`, and
  `owner` continue to run without the Automic Vault credential provider.
