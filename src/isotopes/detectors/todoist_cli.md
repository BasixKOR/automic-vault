# Todoist CLI Radioisotope

The Todoist CLI stores the Todoist API token in
`~/.config/todoist/config.json`. Its sync cache can also contain the same user
token after a successful sync.

The radioisotope migrates that token into the Automic Vault keychain and runs
`todoist` with temporary XDG config/cache directories. The wrapper injects the
token through `TODOIST_TOKEN` only while the command runs and removes temporary
files on exit.

The migration preserves non-secret config values when possible, but runtime
changes to temporary config/cache files are not persisted back to keychain.
