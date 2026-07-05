# sqlcmd

`sqlcmd` stores modern CLI contexts in `~/.sqlcmd/sqlconfig`. On macOS the
upstream code documents Keychain encryption as a TODO, so password fields can
remain in the user config file.

This radioisotope migrates `~/.sqlcmd/sqlconfig` to the keychain when it
contains password fields and wraps `sqlcmd` so the config is recreated under a
temporary home while the CLI runs.
