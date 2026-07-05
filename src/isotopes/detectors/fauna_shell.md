# fauna-shell

This radioisotope covers the `fauna-shell` Homebrew formula.

fauna-shell stores account keys and database secrets in JSON files under
`~/.fauna/credentials`. The migration stores the credential files in the macOS
keychain when known secret-bearing fields are present.

The post-install wrapper runs the real `fauna` launcher with a temporary
`HOME`, recreates `~/.fauna/credentials/account_keys` and
`~/.fauna/credentials/secret_keys` from the keychain values, and removes the
temporary directory when the command exits.

Runtime edits made by `fauna` are intentionally temporary. Re-run migration
after making credential changes outside the isotope wrapper.
