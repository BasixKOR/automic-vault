# mycli

`mycli` reads user configuration from `~/.myclirc` and
`~/.config/mycli/myclirc`. Those files can contain password-bearing DSN aliases,
connection passwords, and SSH tunnel passwords.

This radioisotope migrates those config files to the keychain and wraps `mycli`
so they are recreated under a temporary home/config tree while the CLI runs.
