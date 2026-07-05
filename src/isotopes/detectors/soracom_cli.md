# soracom-cli

This radioisotope covers the `soracom/soracom-cli/soracom-cli` Homebrew
formula.

soracom-cli stores profile authentication material in JSON files under
`~/.soracom`. The migration stores the default profile in the macOS keychain
when known secret-bearing fields are present.

The post-install wrapper runs the real `soracom` launcher with a temporary
`HOME`, recreates `~/.soracom/default.json` from the keychain value, and removes
the temporary directory when the command exits.

Runtime edits made by `soracom` are intentionally temporary. Re-run migration
after making profile changes outside the isotope wrapper.
