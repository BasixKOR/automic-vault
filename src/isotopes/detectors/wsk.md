# wsk

The OpenWhisk CLI reads its default configuration from `~/.wskprops`. The
`AUTH` property in that file is an API authorization key and should not remain
in plaintext on disk.

This radioisotope migrates the default `AUTH` value into Automic Vault-backed
keychain storage, removes the plaintext `AUTH` line from `~/.wskprops`, and
wraps the installed `wsk` launcher so `WHISK_AUTH` is injected only for the
duration of the command.

Non-secret OpenWhisk properties such as `APIHOST`, `APIVERSION`, and namespace
settings remain in `~/.wskprops` so normal CLI behavior is preserved.

## Caveats

- Only the default `~/.wskprops` file is migrated.
- Commands that pass `--auth` or set `WSK_CONFIG_FILE` can override this
  isotope's injected `WHISK_AUTH` value.
- Direct execution of `wsk.av-orig` bypasses Automic Vault injection.
