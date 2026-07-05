# MySQL Client Radioisotope

MySQL Client tools read option files such as `~/.my.cnf`. Those files often
contain plaintext `password = ...` entries for the `[client]` group.

This radioisotope migrates the default user option file into the macOS keychain
and wraps common MySQL client launchers so they receive the option file through
a temporary `--defaults-extra-file` while they run.

## Caveats

- We currently migrate the default `~/.my.cnf` file only.
- Existing option files are not merged with generated temporary files.
- Direct execution of the original binaries will not receive credentials.
