# plumber

This radioisotope covers the `streamdal/public/plumber` Homebrew formula.

Plumber stores local configuration in `~/.batchsh/plumber.json`. The config can
include Streamdal tokens and messaging-system connection secrets such as broker
passwords, relay tokens, API tokens, and embedded credentials.

The migration stores the full Plumber config JSON in the macOS keychain when a
known secret-bearing field is present. The post-install wrapper runs the real
`plumber` binary with a temporary `HOME`, recreates `~/.batchsh/plumber.json`
from the keychain value, and removes the temporary directory when the command
exits.

Runtime edits made by `plumber` are intentionally temporary. Re-run migration
after making config changes outside the isotope wrapper.
