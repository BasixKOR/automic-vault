# isotope:gcli

`gcli` stores forge API tokens in its config file, normally
`~/.config/gcli/config`.

This radioisotope migrates that config into the Automic Vault keychain and
removes plaintext `token = ...` entries from the default config file. The
installed launcher restores the protected config to a temporary file and runs
the original `gcli` with `--config` pointing at that file.

## Caveats

- Runtime config changes are not persisted back to the keychain.
- Project-local `.gcli` files are not migrated.
- Running `gcli.av-orig` directly bypasses the injected config.
